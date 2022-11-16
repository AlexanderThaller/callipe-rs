use axum::extract::Query;
use prometheus::{
    Encoder,
    Registry,
    TextEncoder,
};
use serde::Deserialize;

pub(crate) mod cpu;
pub(crate) mod load;
pub(crate) mod memory;
pub(crate) mod swap;

#[derive(Debug, Deserialize)]
pub(crate) struct Params {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    load::Load::run(&registry).unwrap();
    cpu::Cpu::run(&registry).unwrap();
    memory::Memory::run(&registry).unwrap();
    // TODO: Not working on freebsd
    // swap::Swap::run(&registry).unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}
