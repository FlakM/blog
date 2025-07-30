use anyhow::Result;
use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::info;
use tracing_opentelemetry;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

/// Initialize the observability stack (tracing, metrics, logging)
pub fn init_observability() -> Result<()> {
    // Get configuration from environment variables
    let service_name = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "blog-backend".to_string());
    let service_version = std::env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    let enable_prometheus = std::env::var("ENABLE_PROMETHEUS_METRICS")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    info!(
        service_name = %service_name,
        service_version = %service_version,
        otlp_endpoint = %otlp_endpoint,
        enable_prometheus = %enable_prometheus,
        log_format = %log_format,
        "Initializing observability stack"
    );

    // Create resource describing this service
    let resource = Resource::new(vec![
        SERVICE_NAME.string(service_name.clone()),
        SERVICE_VERSION.string(service_version.clone()),
    ]);

    // Initialize metrics
    if enable_prometheus {
        init_prometheus_metrics()?;
    }
    init_otlp_metrics(&otlp_endpoint, resource.clone())?;

    // Initialize tracing subscriber with optional OpenTelemetry layer
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "blog_backend=debug,sqlx=info,tower_http=debug,axum::rejection=trace".into()
    });

    // Create base subscriber layers
    let registry = tracing_subscriber::registry().with(env_filter);

    // Initialize OTLP tracing if endpoint is configured
    if !otlp_endpoint.is_empty() && otlp_endpoint != "disabled" {
        let tracer = init_tracer(&otlp_endpoint, resource.clone())?;
        info!("OTLP tracer initialized for endpoint: {}", otlp_endpoint);

        // Add OpenTelemetry layer
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        // Choose between JSON and human-readable format with OpenTelemetry
        if log_format.to_lowercase() == "json" {
            registry
                .with(telemetry_layer)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_target(true)
                        .with_current_span(true)
                        .with_span_list(false)
                        .flatten_event(true)
                        .with_span_events(FmtSpan::CLOSE | FmtSpan::NEW),
                )
                .init();
        } else {
            registry
                .with(telemetry_layer)
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_line_number(true)
                        .with_span_events(FmtSpan::CLOSE),
                )
                .init();
        }
    } else {
        // No OpenTelemetry, just regular logging
        if log_format.to_lowercase() == "json" {
            registry
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_target(true)
                        .with_current_span(true)
                        .with_span_list(false)
                        .flatten_event(true)
                        .with_span_events(FmtSpan::CLOSE | FmtSpan::NEW),
                )
                .init();
        } else {
            registry
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_line_number(true)
                        .with_span_events(FmtSpan::CLOSE),
                )
                .init();
        }
    }

    info!("Observability stack initialized successfully");
    Ok(())
}

fn init_tracer(otlp_endpoint: &str, resource: Resource) -> Result<Tracer> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(trace::config().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracer)
}

fn init_prometheus_metrics() -> Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(([127, 0, 0, 1], 9090))
        .install()?;

    info!("Prometheus metrics server started on http://127.0.0.1:9090/metrics");

    Ok(())
}

fn init_otlp_metrics(_otlp_endpoint: &str, _resource: Resource) -> Result<()> {
    // For now, skip OTLP metrics as the API has changed
    // We'll focus on Prometheus metrics and traces
    info!("OTLP metrics skipped for now - using Prometheus instead");
    Ok(())
}

/// Shutdown observability providers gracefully
pub fn shutdown_observability() {
    info!("Shutting down observability providers");
    global::shutdown_tracer_provider();
    // Note: metrics providers don't have a global shutdown in the current version
}
