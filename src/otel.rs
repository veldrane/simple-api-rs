use crate::prelude::*;

/// Errors that can keep OpenTelemetry tracing from starting. `Disabled`
/// isn't really a failure — it's what `init()` returns when
/// `OtelConfig.enabled` is false, so the caller can skip attaching the
/// `OpenTelemetryTracing` middleware the same way it already skips
/// `FaultInject` on an invalid config (see `with_opt`/`with_result` in
/// app.rs).
pub enum OtelError {
    Disabled,
    InvalidEndpoint,
    InvalidTimeout,
    ExporterInit(String),
    SubscriberAlreadySet,
}

impl Into<String> for OtelError {
    fn into(self) -> String {
        match self {
            OtelError::Disabled => "disabled via config".into(),
            OtelError::InvalidEndpoint => "invalid OTLP endpoint".into(),
            OtelError::InvalidTimeout => "invalid exporter timeout".into(),
            OtelError::ExporterInit(e) => format!("failed to initialize OTLP exporter: {e}"),
            OtelError::SubscriberAlreadySet => "global tracing subscriber already set".into(),
        }
    }
}

/// Initializes OpenTelemetry tracing for the whole process, driven by
/// `OtelConfig` (see config.rs):
/// - registers the W3C trace-context propagator (so incoming `traceparent`
///   headers are picked up, and outgoing calls can carry it onward),
/// - wires a batch OTLP/gRPC exporter,
/// - sets `tracing_subscriber`'s global default so plain `tracing::*` spans
///   created anywhere in the app (handlers, middleware) are exported as
///   OpenTelemetry spans too, without touching the OTel API directly.
///
/// Returns `Err(OtelError::Disabled)` when `config.enabled` is false, so
/// the caller can skip attaching `OpenTelemetryTracing` entirely — see
/// `app::builder()`, which treats this exactly like `FaultInject`'s
/// validation failures.
///
/// The returned [`SdkTracer`] only needs to be handed to poem's
/// [`OpenTelemetryTracing`] middleware, which creates the per-request root
/// span and makes it the ambient OTel context. Any `tracing::info_span!`/
/// `trace_span!` created afterwards (in handlers or other middleware) is
/// automatically linked as a child of it — see `fault_inject.rs` /
/// `articles.rs` for the pattern. IMPORTANT: `OpenTelemetryTracing` must be
/// the OUTERMOST middleware for that to work (see the comment above `api`
/// in app.rs, and tests/otel_trace_shape.rs).
pub fn init(config: &OtelConfig) -> Result<SdkTracer, OtelError> {
    if !config.enabled {
        return Err(OtelError::Disabled);
    }
    if config.endpoint.is_empty() {
        return Err(OtelError::InvalidEndpoint);
    }
    if config.timeout == 0 {
        return Err(OtelError::InvalidTimeout);
    }

    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name(config.service_name.clone())
                .build(),
        )
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint(&config.endpoint)
                .with_timeout(Duration::from_secs(config.timeout))
                .build()
                .map_err(|e| OtelError::ExporterInit(e.to_string()))?,
        )
        .build();

    let tracer = provider.tracer("simple-api-trace");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let subscriber = tracing_subscriber::Registry::default().with(otel_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| OtelError::SubscriberAlreadySet)?;

    Ok(tracer)
}
