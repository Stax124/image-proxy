use std::time::Instant;

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry};

/// Application Prometheus metrics shared across handlers.
#[derive(Clone)]
pub struct AppMetrics {
    /// Total image requests: labels `format`, `status`, `path`.
    pub request_count: IntCounterVec,
    /// Handler-wall-time latency in seconds: labels `format`, `status`, `path`.
    pub request_duration: HistogramVec,
    /// Pipeline step latency in seconds: labels `step`, `format`.
    pub pipeline_duration: HistogramVec,
    /// Bytes served in successful responses: labels `format`, `path`.
    pub response_bytes: IntCounterVec,
    /// In-flight image requests (handler entered, not yet finished).
    pub in_flight: IntGauge,
}

/// Closed vocabulary for the Prometheus `path` label (how a request was handled).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestPath {
    Rejected,
    CacheHit,
    PassThrough,
    Transform,
    NonProcessable,
    Fallback,
    FallbackTransform,
    NotFound,
    Unknown,
}

impl RequestPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::CacheHit => "cache_hit",
            Self::PassThrough => "pass_through",
            Self::Transform => "transform",
            Self::NonProcessable => "non_processable",
            Self::Fallback => "fallback",
            Self::FallbackTransform => "fallback_transform",
            Self::NotFound => "not_found",
            Self::Unknown => "unknown",
        }
    }
}

/// Closed vocabulary for the Prometheus `status` label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestStatus {
    Ok,
    NotFound,
    UnsupportedMediaType,
    Error,
    BadGateway,
}

impl RequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotFound => "not_found",
            Self::UnsupportedMediaType => "unsupported_media_type",
            Self::Error => "error",
            Self::BadGateway => "bad_gateway",
        }
    }
}

/// Closed `format` label values used on validation rejects.
///
/// Reject paths must not record the raw URL extension or query `format=` string —
/// those are attacker-controlled and would explode Prometheus series cardinality.
/// Keep the real value in logs / response bodies only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectFormat {
    /// Missing or unusable extension.
    Unknown,
    /// Input extension not in the allow-list, or requested output format not allowed.
    Unsupported,
}

impl RejectFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
        }
    }
}

pub fn setup_metrics() -> (Registry, AppMetrics) {
    let prometheus_registry = Registry::new();

    let request_count = IntCounterVec::new(
        Opts::new(
            "image_requests_total",
            "Total number of requests to the image transformation endpoint",
        ),
        &["format", "status", "path"],
    )
    .expect("failed to create request count counter");

    let request_duration = HistogramVec::new(
        HistogramOpts::new(
            "image_request_duration_seconds",
            "Handler wall time from entry until HttpResponse is returned (streaming paths exclude body transfer)",
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            30.0,
        ]),
        &["format", "status", "path"],
    )
    .expect("failed to create request duration histogram");

    let pipeline_duration = HistogramVec::new(
        HistogramOpts::new(
            "image_pipeline_step_duration_seconds",
            "Time spent on each image pipeline transformation step",
        )
        .buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ]),
        &["step", "format"],
    )
    .expect("failed to create pipeline duration histogram");

    let response_bytes = IntCounterVec::new(
        Opts::new(
            "image_response_bytes_total",
            "Bytes in successful responses (declared size for streaming pass-through; skip if unknown)",
        ),
        &["format", "path"],
    )
    .expect("failed to create response bytes counter");

    let in_flight = IntGauge::new(
        "image_requests_in_flight",
        "Number of image requests currently being handled",
    )
    .expect("failed to create in-flight gauge");

    prometheus_registry
        .register(Box::new(request_count.clone()))
        .expect("failed to register request count counter");
    prometheus_registry
        .register(Box::new(request_duration.clone()))
        .expect("failed to register request duration histogram");
    prometheus_registry
        .register(Box::new(pipeline_duration.clone()))
        .expect("failed to register pipeline duration histogram");
    prometheus_registry
        .register(Box::new(response_bytes.clone()))
        .expect("failed to register response bytes counter");
    prometheus_registry
        .register(Box::new(in_flight.clone()))
        .expect("failed to register in-flight gauge");

    // Process metrics (CPU, RSS, open FDs) on Linux.
    #[cfg(target_os = "linux")]
    {
        let process = prometheus::process_collector::ProcessCollector::for_self();
        if let Err(e) = prometheus_registry.register(Box::new(process)) {
            tracing::debug!("process collector not registered: {}", e);
        }
    }

    let metrics = AppMetrics {
        request_count,
        request_duration,
        pipeline_duration,
        response_bytes,
        in_flight,
    };

    (prometheus_registry, metrics)
}

/// Records request metrics exactly once when dropped (or when [`finish`](Self::finish) is called).
///
/// Defaults: `format=unknown`, `status=error`, `path=unknown`. Use [`reject`], [`ok`], or
/// [`fail`] before the guard is dropped so error early-returns still produce accurate labels.
pub struct RequestTracker {
    metrics: AppMetrics,
    start: Instant,
    format: String,
    status: RequestStatus,
    path: RequestPath,
    bytes: u64,
    finished: bool,
}

impl RequestTracker {
    pub fn new(metrics: AppMetrics) -> Self {
        metrics.in_flight.inc();
        Self {
            metrics,
            start: Instant::now(),
            format: "unknown".to_string(),
            status: RequestStatus::Error,
            path: RequestPath::Unknown,
            bytes: 0,
            finished: false,
        }
    }

    pub fn set_format(&mut self, format: impl Into<String>) {
        self.format = format.into();
    }

    /// Validation / disallowed-format rejection.
    ///
    /// `format` is a closed sentinel ([`RejectFormat`]); never pass raw path extensions.
    pub fn reject(&mut self, format: RejectFormat, status: RequestStatus) {
        self.format = format.as_str().to_string();
        self.status = status;
        self.path = RequestPath::Rejected;
        self.bytes = 0;
    }

    /// Successful response. `bytes` is body length or declared Content-Length; use `0` when unknown.
    pub fn ok(&mut self, path: RequestPath, bytes: u64) {
        self.status = RequestStatus::Ok;
        self.path = path;
        self.bytes = bytes;
    }

    /// Error terminal (status and path chosen by the caller).
    pub fn fail(&mut self, path: RequestPath, status: RequestStatus) {
        self.status = status;
        self.path = path;
        self.bytes = 0;
    }

    /// Explicit finish (also invoked from `Drop`). Safe to call once.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.metrics.in_flight.dec();

        let labels = &[
            self.format.as_str(),
            self.status.as_str(),
            self.path.as_str(),
        ];
        self.metrics.request_count.with_label_values(labels).inc();
        self.metrics
            .request_duration
            .with_label_values(labels)
            .observe(self.start.elapsed().as_secs_f64());

        if self.bytes > 0 && self.status == RequestStatus::Ok {
            self.metrics
                .response_bytes
                .with_label_values(&[self.format.as_str(), self.path.as_str()])
                .inc_by(self.bytes);
        }
    }
}

impl Drop for RequestTracker {
    fn drop(&mut self) {
        self.finish();
    }
}
