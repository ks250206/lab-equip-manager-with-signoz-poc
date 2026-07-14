use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::SdkMeterProvider, propagation::TraceContextPropagator, resource::Resource,
    trace::SdkTracerProvider,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct TelemetryHandles {
    pub tracer_provider: SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
}

pub fn init_telemetry(service_name: &str, endpoint: &str) -> anyhow::Result<TelemetryHandles> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .with_attribute(KeyValue::new("service.namespace", "signozpoc"))
        .build();

    let tracer_provider = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map(|exporter| {
            SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource.clone())
                .build()
        })?;

    let meter_provider = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map(|exporter| {
            SdkMeterProvider::builder()
                .with_periodic_exporter(exporter)
                .with_resource(resource)
                .build()
        })?;

    opentelemetry::global::set_meter_provider(meter_provider.clone());

    let tracer = tracer_provider.tracer("equipment_reservation");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true),
        )
        .with(otel_layer)
        .init();

    Ok(TelemetryHandles {
        tracer_provider,
        meter_provider,
    })
}

pub fn shutdown(handles: TelemetryHandles) {
    let _ = handles.tracer_provider.shutdown();
    let _ = handles.meter_provider.shutdown();
}
