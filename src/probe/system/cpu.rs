use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_int_counter_with_registry,
    register_int_gauge_with_registry,
    Encoder,
    Registry,
    TextEncoder,
};
use serde::Deserialize;
use systemstat::{
    CpuTime,
    Platform,
    System,
};

#[derive(Debug, Deserialize)]
pub(crate) struct Params {}

#[derive(Debug)]
pub(super) struct Cpu {}

#[allow(clippy::unused_async)]
pub(crate) async fn handler(Query(_params): Query<Params>) -> Vec<u8> {
    let registry = Registry::new();
    Cpu::run(&registry).unwrap();

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}

impl Cpu {
    pub(super) fn run(registry: &Registry) -> Result<(), Error> {
        let sys = System::new();
        let cpu = sys.cpu_time()?;
        let cpu = cpu.into_iter().fold(
            CpuTime {
                user: 0,
                nice: 0,
                system: 0,
                interrupt: 0,
                idle: 0,
                other: 0,
            },
            |acc, x| CpuTime {
                user: acc.user + x.user,
                nice: acc.nice + x.nice,
                system: acc.system + x.system,
                interrupt: acc.interrupt + x.interrupt,
                idle: acc.idle + x.idle,
                other: acc.other + x.other,
            },
        );

        register_int_counter_with_registry!("system_cpu_user", "system cpu user usage", registry)?
            .inc_by(cpu.user.try_into().unwrap());

        register_int_counter_with_registry!(
            "system_cpu_nice",
            "system cpu
        nice usage",
            registry
        )?
        .inc_by(cpu.nice.try_into().unwrap());

        register_int_counter_with_registry!(
            "system_cpu_system",
            "system cpu system usage",
            registry
        )?
        .inc_by(cpu.system.try_into().unwrap());

        register_int_counter_with_registry!(
            "system_cpu_irq",
            "system cpu irq
        usage",
            registry
        )?
        .inc_by(cpu.interrupt.try_into().unwrap());

        register_int_counter_with_registry!(
            "system_cpu_idle",
            "system cpu
        idle usage",
            registry
        )?
        .inc_by(cpu.idle.try_into().unwrap());

        register_int_gauge_with_registry!(
            "system_cpu_core_count",
            "how many cpus are available",
            registry
        )?
        .set(num_cpus::get() as i64);

        Ok(())
    }
}
