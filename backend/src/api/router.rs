use axum::{
    http::HeaderValue,
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    api::{auth, equipment, otel_middleware, reservations},
    state::AppState,
};

pub fn app_router(state: AppState) -> Router {
    let origins: Vec<HeaderValue> = state
        .config
        .frontend_origin
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(AllowOrigin::list(origins));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/api/equipment",
            get(equipment::list_equipment).post(equipment::create_equipment),
        )
        .route(
            "/api/equipment/{id}",
            get(equipment::get_equipment)
                .patch(equipment::update_equipment)
                .delete(equipment::delete_equipment),
        )
        .route(
            "/api/equipment/{id}/image",
            post(equipment::upload_equipment_image),
        )
        .route(
            "/api/reservations",
            get(reservations::list_my_reservations).post(reservations::create_reservation),
        )
        .route(
            "/api/reservations/{id}/cancel",
            post(reservations::cancel_reservation),
        )
        .route("/api/demo/slow", get(reservations::slow_probe))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            otel_middleware::otel_http_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
