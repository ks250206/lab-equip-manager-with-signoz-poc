use opentelemetry::{
    logs::{LogRecord as _, Logger as _, LoggerProvider as _, Severity},
    metrics::{Counter, Histogram},
    trace::{TraceContextExt as _, TracerProvider as _},
    Key, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    logs::{SdkLogRecord, SdkLoggerProvider},
    metrics::SdkMeterProvider,
    propagation::TraceContextPropagator,
    resource::Resource,
    trace::SdkTracerProvider,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{
    fmt,
    layer::{Context as LayerContext, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

#[derive(Clone)]
pub struct AppMetrics {
    request_count: Counter<u64>,
    request_duration: Histogram<f64>,
}

impl AppMetrics {
    pub fn new() -> Self {
        let meter = opentelemetry::global::meter("equipment_reservation.http");
        Self {
            request_count: meter
                .u64_counter("http.server.request.count")
                .with_description("Number of HTTP requests handled by the API")
                .build(),
            request_duration: meter
                .f64_histogram("http.server.request.duration")
                .with_unit("s")
                .with_description("Duration of HTTP requests handled by the API")
                .build(),
        }
    }

    pub fn record_request(&self, method: &str, route: &str, status_code: u16, duration_secs: f64) {
        let attributes = [
            KeyValue::new("http.request.method", method.to_owned()),
            KeyValue::new("http.route", route.to_owned()),
            KeyValue::new("http.response.status_code", i64::from(status_code)),
        ];
        self.request_count.add(1, &attributes);
        self.request_duration.record(duration_secs, &attributes);
    }
}

struct OtelLogLayer {
    provider: SdkLoggerProvider,
}

impl OtelLogLayer {
    fn new(provider: SdkLoggerProvider) -> Self {
        Self { provider }
    }
}

struct EventVisitor<'a> {
    record: &'a mut SdkLogRecord,
}

impl tracing::field::Visit for EventVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.record.set_body(format!("{value:?}").into());
        } else {
            self.record
                .add_attribute(Key::new(field.name()), format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.record.set_body(value.to_owned().into());
        } else {
            self.record
                .add_attribute(Key::new(field.name()), value.to_owned());
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.record.add_attribute(Key::new(field.name()), value);
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.record.add_attribute(Key::new(field.name()), value);
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.record
            .add_attribute(Key::new(field.name()), value.to_string());
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.record.add_attribute(Key::new(field.name()), value);
    }
}

impl<S> Layer<S> for OtelLogLayer
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: LayerContext<'_, S>) {
        let metadata = event.metadata();
        let logger = self.provider.logger("equipment_reservation");
        let mut record = logger.create_log_record();
        record.set_event_name(metadata.name());
        record.set_target(metadata.target().to_owned());
        record.set_severity_text(metadata.level().as_str());
        record.set_severity_number(match *metadata.level() {
            tracing::Level::TRACE => Severity::Trace,
            tracing::Level::DEBUG => Severity::Debug,
            tracing::Level::INFO => Severity::Info,
            tracing::Level::WARN => Severity::Warn,
            tracing::Level::ERROR => Severity::Error,
        });
        event.record(&mut EventVisitor {
            record: &mut record,
        });

        let context = tracing::Span::current().context();
        let span = context.span();
        let span_context = span.span_context();
        if span_context.is_valid() {
            record.set_trace_context(
                span_context.trace_id(),
                span_context.span_id(),
                Some(span_context.trace_flags()),
            );
        }
        logger.emit(record);
    }
}

pub struct TelemetryHandles {
    pub tracer_provider: SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
    pub logger_provider: SdkLoggerProvider,
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
                .with_resource(resource.clone())
                .build()
        })?;

    opentelemetry::global::set_meter_provider(meter_provider.clone());

    let logger_provider = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map(|exporter| {
            SdkLoggerProvider::builder()
                .with_batch_exporter(exporter)
                .with_resource(resource.clone())
                .build()
        })?;

    let tracer = tracer_provider.tracer("equipment_reservation");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Do not feed OTLP exporter transport diagnostics back into the OTLP log
    // pipeline. Otherwise exporter failures can amplify their own logs.
    let env_filter = ["hyper=off", "tonic=off", "h2=off", "reqwest=off"]
        .into_iter()
        .fold(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            |filter, directive| filter.add_directive(directive.parse().expect("valid directive")),
        );

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true),
        )
        .with(OtelLogLayer::new(logger_provider.clone()))
        .with(otel_layer)
        .init();

    Ok(TelemetryHandles {
        tracer_provider,
        meter_provider,
        logger_provider,
    })
}

pub fn shutdown(handles: TelemetryHandles) {
    let _ = handles.tracer_provider.shutdown();
    let _ = handles.meter_provider.shutdown();
    let _ = handles.logger_provider.shutdown();
}
