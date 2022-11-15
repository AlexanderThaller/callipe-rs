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

    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 6122);
    let incoming_v4 = AddrIncoming::bind(&localhost_v4).unwrap();

    let localhost_v6 = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 6122);
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
            target: Target,
            count: Option<NonZeroU32>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum Target {
            Addr(IpAddr),
            Hostname(String),
        }

        #[derive(Debug)]
        struct Pinger {
            target: Target,
            count: NonZeroU32,
        }

        #[derive(Debug, Default, PartialEq)]
        struct Ping {
            transmitted: Option<u32>,
            received: Option<u32>,
            packet_loss: Option<f64>,
            time: Option<u32>,
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

                let output = Command::new("ping")
                    .arg("-q")
                    .arg("-c")
                    .arg(format!("{}", self.count))
                    .arg(format!("{}", self.target))
                    .output()
                    .await
                    .unwrap();

                register_int_gauge_with_registry!(
                    "probe_ping_failed_status",
                    "if the probe failed",
                    registry
                )
                .unwrap()
                .set(output.status.code().unwrap_or_default().into());

                let ping = Ping::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();

                if let Some(transmitted) = ping.transmitted {
                    register_int_gauge_with_registry!(
                        "probe_ping_transmitted_count",
                        "how many pings where sent",
                        registry
                    )
                    .unwrap()
                    .set(transmitted.into());
                }

                if let Some(received) = ping.received {
                    register_int_gauge_with_registry!(
                        "probe_ping_received_count",
                        "how many pings where received",
                        registry
                    )
                    .unwrap()
                    .set(received.into());
                }

                if let Some(packet_loss) = ping.packet_loss {
                    register_gauge_with_registry!(
                        "probe_ping_packetloss_precent",
                        "percentage of lost pings",
                        registry
                    )
                    .unwrap()
                    .set(packet_loss);
                }

                if let Some(time) = ping.time {
                    register_int_gauge_with_registry!(
                        "probe_ping_time_milliseconds",
                        "how long pinging took in total",
                        registry
                    )
                    .unwrap()
                    .set(time.into());
                }

                if let Some(min) = ping.min {
                    register_gauge_with_registry!(
                        "probe_ping_min_milliseconds",
                        "minimum duration of pings",
                        registry
                    )
                    .unwrap()
                    .set(min);
                }

                if let Some(avg) = ping.avg {
                    register_gauge_with_registry!(
                        "probe_ping_avg_milliseconds",
                        "average duration of pings",
                        registry
                    )
                    .unwrap()
                    .set(avg);
                }

                if let Some(max) = ping.max {
                    register_gauge_with_registry!(
                        "probe_ping_max_milliseconds",
                        "maximum duration of pings",
                        registry
                    )
                    .unwrap()
                    .set(max);
                }

                if let Some(mdev) = ping.mdev {
                    register_gauge_with_registry!(
                        "probe_ping_mdev_milliseconds",
                        "standard deviation of pings",
                        registry
                    )
                    .unwrap()
                    .set(mdev);
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
                        | ["PING" | "PING6(56=40+8+8", ..]
                        | ["---", _, "ping" | "ping6", "statistics", "---"] => {}

                        [transmitted, "packets", "transmitted,", received, "packets", "received,", packet_loss, "packet", "loss"] =>
                        {
                            out.transmitted = Some(transmitted.parse().unwrap());
                            out.received = Some(received.parse().unwrap());
                            out.packet_loss =
                                Some(packet_loss.trim_end_matches('%').parse().unwrap());
                        }

                        [transmitted, "packets", "transmitted,", received, "received,", packet_loss, "packet", "loss,", "time", time] =>
                        {
                            out.transmitted = Some(transmitted.parse().unwrap());
                            out.received = Some(received.parse().unwrap());
                            out.packet_loss =
                                Some(packet_loss.trim_end_matches('%').parse().unwrap());
                            out.time = Some(time.trim_end_matches("ms").parse().unwrap());
                        }

                        ["rtt", "min/avg/max/mdev", "=", data, "ms"]
                        | ["round-trip", "min/avg/max/stddev" | "min/avg/max/std-dev", "=", data, "ms"] =>
                        {
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

        impl std::fmt::Display for Target {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Addr(v) => write!(f, "{v}"),
                    Self::Hostname(v) => write!(f, "{v}"),
                }
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
                            transmitted: Some(1),
                            received: Some(1),
                            packet_loss: Some(0.0),
                            time: Some(0),
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
                            transmitted: Some(10),
                            received: Some(10),
                            packet_loss: Some(0.0),
                            time: Some(9011),
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
                            transmitted: Some(10),
                            received: Some(0),
                            packet_loss: Some(100.0),
                            time: Some(9244),
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
