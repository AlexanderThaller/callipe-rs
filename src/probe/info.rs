use prometheus::{
    register_int_gauge_vec_with_registry,
    Encoder,
    Registry,
    TextEncoder,
};

#[allow(clippy::unused_async)]
pub(crate) async fn handler() -> Vec<u8> {
    let registry = Registry::new();

    register_int_gauge_vec_with_registry!(
        "info",
        "information abouot callipe-rs",
        &[
            "build_semver",
            "build_timestamp",
            "git_semver",
            "git_branch",
            "git_sha"
        ],
        registry
    )
    .unwrap()
    .with_label_values(&[
        env!("VERGEN_BUILD_SEMVER"),
        env!("VERGEN_BUILD_TIMESTAMP"),
        env!("VERGEN_GIT_SEMVER"),
        env!("VERGEN_GIT_BRANCH"),
        env!("VERGEN_GIT_SHA"),
    ])
    .set(1);

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    buffer
}
