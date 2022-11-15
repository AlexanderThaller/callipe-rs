use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_int_gauge_with_registry,
    Encoder,
    Registry,
    TextEncoder,
};
use serde::Deserialize;
use systemstat::{
    Platform,
    System,
};

#[derive(Debug, Deserialize)]
pub(crate) struct Params {}

#[derive(Debug)]
pub(super) struct Swap {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    Swap::run(&registry).unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}

impl Swap {
    pub(super) fn run(registry: &Registry) -> Result<(), Error> {
        let sys = System::new();
        let swap = sys.swap()?;

        dbg!(&swap);

        #[allow(clippy::cast_possible_wrap)]
        register_int_gauge_with_registry!(
            "system_swap_total_byte",
            "total swap in the system",
            registry
        )?
        .set(swap.total.0 as i64);

        #[allow(clippy::cast_possible_wrap)]
        register_int_gauge_with_registry!(
            "system_swap_free_byte",
            "free swap in the system",
            registry
        )?
        .set(swap.free.0 as i64);

        Ok(())
    }
}
