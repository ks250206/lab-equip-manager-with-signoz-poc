use std::time::Instant;

use axum::{
    body::Body,
    extract::{MatchedPath, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_http::HeaderExtractor;
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::state::AppState;

pub async fn otel_http_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| "<unmatched>".to_owned());
    let parent_cx = extract_context(req.headers());
    let started_at = Instant::now();

    let tracer = global::tracer("equipment_reservation");
    let otel_span = tracer
        .span_builder(format!("{method} {path}"))
        .with_kind(opentelemetry::trace::SpanKind::Server)
        .with_attributes([
            KeyValue::new("http.request.method", method.to_string()),
            KeyValue::new("url.path", path.clone()),
        ])
        .start_with_context(&tracer, &parent_cx);

    let cx = Context::current_with_span(otel_span);
    let tracing_span = info_span!("http.request", method = %method, path = %path);
    let _ = tracing_span.set_parent(cx);

    let response = next.run(req).instrument(tracing_span).await;
    state.metrics.record_request(
        method.as_str(),
        &route,
        response.status().as_u16(),
        started_at.elapsed().as_secs_f64(),
    );
    response
}

fn extract_context(headers: &HeaderMap) -> Context {
    let extractor = HeaderExtractor(headers);
    global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}
