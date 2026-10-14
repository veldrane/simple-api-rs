use crate::prelude::*;

const OTLP_ENDPOINT: &str = "http://localhost:4317";

/// Initializes OpenTelemetry tracing for the whole process:
/// - registers the W3C trace-context propagator (so incoming `traceparent`
///   headers are picked up, and outgoing calls can carry it onward),
/// - wires a batch OTLP/gRPC exporter,
/// - sets `tracing_subscriber`'s global default so plain `tracing::*` spans
///   created anywhere in the app (handlers, middleware) are exported as
///   OpenTelemetry spans too, without touching the OTel API directly.
///
/// The returned [`SdkTracer`] only needs to be handed to poem's
/// [`OpenTelemetryTracing`] middleware, which creates the per-request root
/// span and makes it the ambient OTel context. Any `tracing::info_span!`/
/// `trace_span!` created afterwards (in handlers or other middleware) is
/// automatically linked as a child of it — see `fault_inject.rs` /
/// `articles.rs` for the pattern.
pub fn init() -> SdkTracer {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("simple-api").build())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_protocol(Protocol::Grpc)
                .with_endpoint(OTLP_ENDPOINT)
                .with_timeout(Duration::from_secs(3))
                .build()
                .expect("Trace exporter should initialize."),
        )
        .build();

    let tracer = provider.tracer("simple-api-trace");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let subscriber = tracing_subscriber::Registry::default().with(otel_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global subscriber.");

    tracer
}
