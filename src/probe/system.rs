use axum::extract::Query;
use prometheus::{
    Encoder,
    Registry,
    TextEncoder,
};
use serde::Deserialize;

pub(crate) mod cpu;
pub(crate) mod load;

#[derive(Debug, Deserialize)]
pub(crate) struct Params {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    load::Load::run(&registry).unwrap();
    cpu::Cpu::run(&registry).await.unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}
