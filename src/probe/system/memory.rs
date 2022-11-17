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

        Memory::platform_memory(registry, memory)?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[allow(clippy::cast_possible_wrap)]
    fn platform_memory(registry: &Registry, memory: systemstat::Memory) -> Result<(), Error> {
        let info = memory.platform_memory.meminfo;

        let platform = register_int_gauge_vec_with_registry!(
            "system_memory_platform_byte",
            "platform specific memory information",
            &["os", "name"],
            registry
        )?;

        for (name, value) in info {
            platform
                .with_label_values(&["linux", &name.to_ascii_lowercase()])
                .set(value.as_u64() as i64);
        }

        Ok(())
    }

    #[cfg(target_os = "freebsd")]
    #[allow(clippy::cast_possible_wrap)]
    fn platform_memory(registry: &Registry, memory: systemstat::Memory) -> Result<(), Error> {
        let info = memory.platform_memory;

        let platform = register_int_gauge_vec_with_registry!(
            "system_memory_platform_byte",
            "platform specific memory information",
            &["os", "name"],
            registry
        )?;

        platform
            .with_label_values(&["freebsd", "active"])
            .set(info.active.as_u64() as i64);

        platform
            .with_label_values(&["freebsd", "inactive"])
            .set(info.inactive.as_u64() as i64);

        platform
            .with_label_values(&["freebsd", "wired"])
            .set(info.wired.as_u64() as i64);

        platform
            .with_label_values(&["freebsd", "cache"])
            .set(info.cache.as_u64() as i64);

        platform
            .with_label_values(&["freebsd", "zfs_arc"])
            .set(info.zfs_arc.as_u64() as i64);

        platform
            .with_label_values(&["freebsd", "free"])
            .set(info.free.as_u64() as i64);

        Ok(())
    }
}
