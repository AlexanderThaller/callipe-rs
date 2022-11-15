use std::time::Duration;

use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_gauge_with_registry,
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
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
pub(crate) struct Params {}

#[derive(Debug)]
pub(super) struct Cpu {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    Cpu::run(&registry).await.unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}

impl Cpu {
    pub(super) async fn run(registry: &Registry) -> Result<(), Error> {
        let sys = System::new();

        let cpu = sys.cpu_load_aggregate()?;
        sleep(Duration::from_secs(1)).await;
        let cpu = cpu.done()?;

        register_gauge_with_registry!("system_cpu_user", "system cpu user usage", registry)?
            .set(cpu.user.into());

        register_gauge_with_registry!("nice_cpu_nice", "nice cpu nice usage", registry)?
            .set(cpu.nice.into());

        register_gauge_with_registry!("system_cpu_system", "system cpu system usage", registry)?
            .set(cpu.system.into());

        register_gauge_with_registry!(
            "system_cpu_interrupt",
            "system cpu interrupt usage",
            registry
        )?
        .set(cpu.interrupt.into());

        register_gauge_with_registry!("idle_cpu_interrupt", "idle cpu interrupt usage", registry)?
            .set(cpu.idle.into());

        #[cfg(target_os = "linux")]
        register_gauge_with_registry!("system_cpu_iowait", "system cpu iowait usage", registry)?
            .set(cpu.platform.iowait.into());

        register_int_gauge_with_registry!(
            "system_cpu_core_count",
            "how many cpus are available",
            registry
        )?
        .set(num_cpus::get() as i64);

        Ok(())
    }
}
