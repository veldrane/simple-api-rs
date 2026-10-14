// Regression test for the middleware ordering in app.rs: `OpenTelemetryTracing`
// must be the OUTERMOST middleware (applied last), otherwise fault_inject's
// span and the handler's span get spliced into a trace that is disconnected
// from the actual per-request root span. Builds the route stack both ways,
// in-process, and inspects the exported spans with an in-memory exporter —
// no real OTLP collector needed. See the comment above `api` in app.rs.
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{InMemorySpanExporterBuilder, SdkTracerProvider, SimpleSpanProcessor};
use poem::test::TestClient;
use simple_api_rs::prelude::*;

fn new_fault_inject(config: &Config) -> FaultInject {
    match FaultInject::try_from(&config.fault_inject) {
        Ok(fi) => fi,
        Err(_) => panic!("fault inject config invalid"),
    }
}

#[tokio::test]
async fn middleware_order_decides_trace_shape() {
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let exporter = InMemorySpanExporterBuilder::new().build();
    let provider = SdkTracerProvider::builder()
        .with_span_processor(SimpleSpanProcessor::new(exporter.clone()))
        .build();
    let tracer = provider.tracer("test");

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let subscriber = tracing_subscriber::Registry::default().with(otel_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // min_delay == max_delay takes fault_inject's deterministic "no delay"
    // branch, sidestepping the (separately tracked) random-delay/error_rate
    // bug — this test only cares about span shape, not fault_inject's math.
    let config = Config::default().with_fault_inject(
        FaultInjectConfig::default().with_min_delay(0).with_max_delay(0),
    );

    // --- scenario A: CURRENT (broken) order — fault_inject wraps OpenTelemetryTracing ---
    {
        let state = AppState::build(&config);
        let fault_inject = new_fault_inject(&config);
        let app = Route::new()
            .at("/api/v1/articles", get(get_articles))
            .with(OpenTelemetryTracing::new(tracer.clone()))
            .with(fault_inject)
            .data(state);

        let cli = TestClient::new(app);
        let resp = cli.get("/api/v1/articles").send().await;
        resp.assert_status_is_ok();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let spans = exporter.get_finished_spans().unwrap();
        println!("\n--- current order (broken) ---");
        for s in &spans {
            println!(
                "name={:<24} trace_id={} span_id={} parent_span_id={}",
                s.name, s.span_context.trace_id(), s.span_context.span_id(), s.parent_span_id
            );
        }
        let trace_ids: std::collections::HashSet<_> =
            spans.iter().map(|s| s.span_context.trace_id().to_string()).collect();
        println!("distinct trace_ids (current order) = {:?}", trace_ids);
        assert!(
            trace_ids.len() > 1,
            "expected current order to split into multiple traces, got {:?}",
            trace_ids
        );
        exporter.reset();
    }

    // --- scenario B: FIXED order — OpenTelemetryTracing wraps everything ---
    {
        let state = AppState::build(&config);
        let fault_inject = new_fault_inject(&config);
        let app = Route::new()
            .at("/api/v1/articles", get(get_articles))
            .with(fault_inject)
            .with(OpenTelemetryTracing::new(tracer.clone()))
            .data(state);

        let cli = TestClient::new(app);
        let resp = cli.get("/api/v1/articles").send().await;
        resp.assert_status_is_ok();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let spans = exporter.get_finished_spans().unwrap();
        println!("\n--- fixed order ---");
        for s in &spans {
            println!(
                "name={:<24} trace_id={} span_id={} parent_span_id={}",
                s.name, s.span_context.trace_id(), s.span_context.span_id(), s.parent_span_id
            );
        }
        let trace_ids: std::collections::HashSet<_> =
            spans.iter().map(|s| s.span_context.trace_id().to_string()).collect();
        println!("distinct trace_ids (fixed order) = {:?}", trace_ids);
        assert_eq!(trace_ids.len(), 1, "expected one shared trace, got {:?}", trace_ids);
        exporter.reset();
    }
}
