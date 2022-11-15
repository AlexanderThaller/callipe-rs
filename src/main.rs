#![warn(clippy::pedantic)]
//#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]
#![forbid(unsafe_code)]

use std::{
    net::{
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
    },
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use axum::{
    routing::get,
    Router,
};
use hyper::server::{
    accept::Accept,
    conn::AddrIncoming,
};

struct CombinedIncoming {
    a: AddrIncoming,
    b: AddrIncoming,
}

impl Accept for CombinedIncoming {
    type Conn = <AddrIncoming as Accept>::Conn;
    type Error = <AddrIncoming as Accept>::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        if let Poll::Ready(Some(value)) = Pin::new(&mut self.a).poll_accept(cx) {
            return Poll::Ready(Some(value));
        }

        if let Poll::Ready(Some(value)) = Pin::new(&mut self.b).poll_accept(cx) {
            return Poll::Ready(Some(value));
        }

        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/probe/ping", get(probe::ping::handler));

    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000);
    let incoming_v4 = AddrIncoming::bind(&localhost_v4).unwrap();

    let localhost_v6 = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 3000);
    let incoming_v6 = AddrIncoming::bind(&localhost_v6).unwrap();

    let combined = CombinedIncoming {
        a: incoming_v4,
        b: incoming_v6,
    };

    axum::Server::builder(combined)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

mod handler {}

mod probe {
    pub(crate) mod ping {
        use std::{
            net::IpAddr,
            num::NonZeroU32,
            str::FromStr,
        };

        use axum::extract::Query;
        use prometheus::{
            register_gauge_with_registry,
            register_int_gauge_with_registry,
            Encoder,
            Registry,
            TextEncoder,
        };
        use serde::Deserialize;
        use tokio::process::Command;

        #[derive(Debug, Deserialize)]
        pub(crate) struct Params {
            target: IpAddr,
            count: Option<NonZeroU32>,
        }

        #[derive(Debug)]
        struct Pinger {
            target: IpAddr,
            count: NonZeroU32,
        }

        #[derive(Debug, Default, PartialEq)]
        struct Ping {
            transmitted: u32,
            received: u32,
            packet_loss: u8,
            time: u32,
            min: Option<f64>,
            avg: Option<f64>,
            max: Option<f64>,
            mdev: Option<f64>,
        }

        pub(crate) async fn handler(Query(params): Query<Params>) -> Vec<u8> {
            let registry = Pinger {
                target: params.target,
                count: params.count.unwrap_or(NonZeroU32::new(1).unwrap()),
            }
            .run()
            .await
            .unwrap();

            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            encoder.encode(&metric_families, &mut buffer).unwrap();

            buffer
        }

        impl Pinger {
            async fn run(self) -> Result<Registry, String> {
                let registry = Registry::new();

                let transmitted = register_int_gauge_with_registry!(
                    "probe_ping_transmitted_count",
                    "how many pings where sent",
                    registry
                )
                .unwrap();

                let received = register_int_gauge_with_registry!(
                    "probe_ping_received_count",
                    "how many pings where received",
                    registry
                )
                .unwrap();

                let packet_loss = register_int_gauge_with_registry!(
                    "probe_ping_packetloss_precent",
                    "percentage of lost pings",
                    registry
                )
                .unwrap();

                let time = register_int_gauge_with_registry!(
                    "probe_ping_time_milliseconds",
                    "how long pinging took in total",
                    registry
                )
                .unwrap();

                let min = register_gauge_with_registry!(
                    "probe_ping_min_milliseconds",
                    "minimum duration of pings",
                    registry
                )
                .unwrap();

                let max = register_gauge_with_registry!(
                    "probe_ping_max_milliseconds",
                    "maximum duration of pings",
                    registry
                )
                .unwrap();

                let avg = register_gauge_with_registry!(
                    "probe_ping_avg_milliseconds",
                    "average duration of pings",
                    registry
                )
                .unwrap();

                let mdev = register_gauge_with_registry!(
                    "probe_ping_mdev_milliseconds",
                    "standard deviation of pings",
                    registry
                )
                .unwrap();

                let output = Command::new("ping")
                    .arg("-q")
                    .arg("-c")
                    .arg(format!("{}", self.count))
                    .arg(format!("{}", self.target))
                    .output()
                    .await
                    .unwrap();

                let ping = Ping::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();

                transmitted.set(ping.transmitted.into());
                received.set(ping.received.into());
                packet_loss.set(ping.packet_loss.into());
                time.set(ping.time.into());

                if let Some(m) = ping.min {
                    min.set(m);
                }

                if let Some(a) = ping.avg {
                    avg.set(a);
                }

                if let Some(m) = ping.max {
                    max.set(m);
                }

                if let Some(m) = ping.mdev {
                    mdev.set(m);
                }

                Ok(registry)
            }
        }

        impl std::str::FromStr for Ping {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut out = Self::default();

                for line in s.lines() {
                    let split = line.split_ascii_whitespace().collect::<Vec<_>>();

                    match split.as_slice() {
                        []
                        | ["PING", ..]
                        | ["---", _, "ping6", "statistics", "---"]
                        | ["---", _, "ping", "statistics", "---"] => {}

                        [transmitted, "packets", "transmitted,", received, "received,", packet_loss, "packet", "loss,", "time", time] =>
                        {
                            out.transmitted = transmitted.parse().unwrap();
                            out.received = received.parse().unwrap();
                            out.packet_loss = packet_loss.trim_end_matches('%').parse().unwrap();
                            out.time = time.trim_end_matches("ms").parse().unwrap();
                        }

                        ["rtt", "min/avg/max/mdev", "=", data, "ms"]
                        | ["round-trip", "min/avg/max/std-dev", "=", data, "ms"] => {
                            let mut split = data.split('/');

                            out.min = split.next().map(str::parse).transpose().unwrap();
                            out.avg = split.next().map(str::parse).transpose().unwrap();
                            out.max = split.next().map(str::parse).transpose().unwrap();
                            out.mdev = split.next().map(str::parse).transpose().unwrap();
                        }

                        other => todo!("{other:?}"),
                    }
                }

                Ok(out)
            }
        }

        #[cfg(test)]
        mod tests {
            mod ping {
                mod parse {
                    use pretty_assertions::assert_eq;
                    use std::str::FromStr;

                    use crate::probe::ping::Ping;

                    #[test]
                    fn single_request() {
                        // ping -q -c 1 1.1.1.1
                        const INPUT: &str = r#"
PING 1.1.1.1 (1.1.1.1) 56(84) bytes of data.

--- 1.1.1.1 ping statistics ---
1 packets transmitted, 1 received, 0% packet loss, time 0ms
rtt min/avg/max/mdev = 7.537/7.537/7.537/0.000 ms"#;

                        let expected = Ping {
                            transmitted: 1,
                            received: 1,
                            packet_loss: 0,
                            time: 0,
                            min: Some(7.537),
                            avg: Some(7.537),
                            max: Some(7.537),
                            mdev: Some(0.0),
                        };

                        let got = Ping::from_str(INPUT).unwrap();

                        dbg!(&got);

                        assert_eq!(expected, got);
                    }

                    #[test]
                    fn ten_request() {
                        // ping -q -c 10 1.1.1.1
                        const INPUT: &str = r#"
PING 1.1.1.1 (1.1.1.1) 56(84) bytes of data.

--- 1.1.1.1 ping statistics ---
10 packets transmitted, 10 received, 0% packet loss, time 9011ms
rtt min/avg/max/mdev = 7.427/7.654/7.936/0.169 ms"#;

                        let expected = Ping {
                            transmitted: 10,
                            received: 10,
                            packet_loss: 0,
                            time: 9011,
                            min: Some(7.427),
                            avg: Some(7.654),
                            max: Some(7.936),
                            mdev: Some(0.169),
                        };

                        let got = Ping::from_str(INPUT).unwrap();

                        dbg!(&got);

                        assert_eq!(expected, got);
                    }

                    #[test]
                    fn ten_request_failing() {
                        // ping -q -c 10 51.61.61.1
                        const INPUT: &str = r#"
PING 51.61.61.1 (51.61.61.1) 56(84) bytes of data.

--- 51.61.61.1 ping statistics ---
10 packets transmitted, 0 received, 100% packet loss, time 9244ms

"#;

                        let expected = Ping {
                            transmitted: 10,
                            received: 0,
                            packet_loss: 100,
                            time: 9244,
                            min: None,
                            avg: None,
                            max: None,
                            mdev: None,
                        };

                        let got = Ping::from_str(INPUT).unwrap();

                        dbg!(&got);

                        assert_eq!(expected, got);
                    }
                }
            }
        }
    }
}
