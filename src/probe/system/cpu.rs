use anyhow::Error;
use axum::extract::Query;
use prometheus::{
    register_int_gauge_with_registry,
    Encoder,
    Registry,
    TextEncoder,
};
use serde::Deserialize;

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
        register_int_gauge_with_registry!(
            "system_cpu_core_count",
            "how many cpus are available",
            registry
        )?
        .set(num_cpus::get() as i64);

        Cpu::set_metrics(registry).await
    }

    #[cfg(target_os = "linux")]
    async fn set_metrics(registry: &Registry) -> Result<(), Error> {
        use prometheus::register_int_counter_with_registry;
        use tokio::{
            fs::File,
            io::{
                AsyncBufReadExt,
                BufReader,
            },
        };

        let file = File::open("/proc/stat").await?;
        let mut reader = BufReader::new(file);
        let mut cpu_line = String::new();
        reader.read_line(&mut cpu_line).await?;

        let mut split = cpu_line.split_ascii_whitespace();
        split.next();

        let user = split.next().unwrap().parse()?;
        register_int_counter_with_registry!("system_cpu_user", "system cpu user usage", registry)?
            .inc_by(user);

        let nice = split.next().unwrap().parse()?;
        register_int_counter_with_registry!("system_cpu_nice", "system cpu nice usage", registry)?
            .inc_by(nice);

        let system = split.next().unwrap().parse()?;
        register_int_counter_with_registry!(
            "system_cpu_system",
            "system cpu system usage",
            registry
        )?
        .inc_by(system);

        let idle = split.next().unwrap().parse()?;
        register_int_counter_with_registry!("system_cpu_idle", "system cpu idle usage", registry)?
            .inc_by(idle);

        let iowait = split.next().unwrap().parse()?;
        register_int_counter_with_registry!(
            "system_cpu_iowait",
            "system cpu iowait usage",
            registry
        )?
        .inc_by(iowait);

        let irq = split.next().unwrap().parse()?;
        register_int_counter_with_registry!("system_cpu_irq", "system cpu irq usage", registry)?
            .inc_by(irq);

        Ok(())
    }
}
