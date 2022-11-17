use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_int_gauge_vec_with_registry,
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
pub(super) struct Memory {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    Memory::run(&registry).unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}

impl Memory {
    #[allow(clippy::cast_possible_wrap)]
    pub(super) fn run(registry: &Registry) -> Result<(), Error> {
        let sys = System::new();

        let memory = sys.memory()?;
        // let info = memory.platform_memory.meminfo;

        dbg!(&memory);

        register_int_gauge_with_registry!(
            "system_memory_total_byte",
            "total memory in the system",
            registry
        )?
        .set(memory.total.0 as i64);

        register_int_gauge_with_registry!(
            "system_memory_free_byte",
            "free memory in the system",
            registry
        )?
        .set(memory.free.0 as i64);

        // let platform = register_int_gauge_vec_with_registry!(
        //    "system_memory_platform_byte",
        //    "platform specific memory information",
        //    &["name"],
        //    registry
        //)?;

        // for (name, value) in info {
        //    platform
        //        .with_label_values(&[&name])
        //        .set(value.as_u64() as i64);
        //}

        Ok(())
    }
}
