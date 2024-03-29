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

mod probe;

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
    let system_routes = Router::new()
        .route("/", get(probe::system::handler))
        .route("/cpu", get(probe::system::cpu::handler))
        .route("/load", get(probe::system::load::handler))
        .route("/memory", get(probe::system::memory::handler))
        .route("/swap", get(probe::system::swap::handler));

    let probe_routes = Router::new()
        .route("/info", get(probe::info::handler))
        .route("/ping", get(probe::ping::handler))
        .nest("/system", system_routes);

    let app = Router::new().nest("/probe", probe_routes);

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
