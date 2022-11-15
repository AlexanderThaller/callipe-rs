use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_gauge_with_registry,
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
pub(super) struct Load {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    Load::run(&registry).unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}

impl Load {
    pub(super) fn run(registry: &Registry) -> Result<(), Error> {
        let sys = System::new();
        let load = sys.load_average()?;

        register_gauge_with_registry!(
            "system_load_1",
            "system load average over 1 minute",
            registry
        )?
        .set(load.one.into());

        register_gauge_with_registry!(
            "system_load_5",
            "system load average over 5 minute",
            registry
        )?
        .set(load.five.into());

        register_gauge_with_registry!(
            "system_load_15",
            "system load average over 15 minute",
            registry
        )?
        .set(load.fifteen.into());

        Ok(())
    }
}
