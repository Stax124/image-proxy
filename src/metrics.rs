use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry};

pub fn setup_metrics() -> (Registry, HistogramVec, IntCounterVec) {
    let prometheus_registry = Registry::new();

    let request_count = IntCounterVec::new(
        Opts::new(
            "image_requests_total",
            "Total number of requests to the image transformation endpoint",
        ),
        &["format", "status"],
    )
    .expect("failed to create request count counter");

    prometheus_registry
        .register(Box::new(request_count.clone()))
        .expect("failed to register request count counter");

    let pipeline_duration = HistogramVec::new(
        HistogramOpts::new(
            "image_pipeline_step_duration_seconds",
            "Time spent on each image pipeline transformation step",
        )
        .buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ]),
        &["step"],
    )
    .expect("failed to create pipeline duration histogram");

    prometheus_registry
        .register(Box::new(pipeline_duration.clone()))
        .expect("failed to register pipeline duration histogram");

    (prometheus_registry, pipeline_duration, request_count)
}
