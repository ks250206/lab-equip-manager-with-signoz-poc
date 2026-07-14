use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    api::extractors::AuthUser, domain::is_valid_range, models::Reservation, state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub equipment_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[instrument(skip(state, body))]
pub async fn create_reservation(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateReservationRequest>,
) -> Response {
    if !is_valid_range(body.starts_at, body.ends_at) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_range"})),
        )
            .into_response();
    }

    let Some(_) = (match state.db.get_equipment(body.equipment_id).await {
        Ok(e) => e,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    match state
        .db
        .create_reservation(body.equipment_id, user.0.id, body.starts_at, body.ends_at)
        .await
    {
        Ok(r) => (StatusCode::CREATED, Json(r)).into_response(),
        Err(err) if is_reservation_conflict(&err) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "reservation_conflict"})),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn is_reservation_conflict(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.code().as_deref() == Some("23P01"))
}

#[instrument(skip(state))]
pub async fn list_my_reservations(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<Reservation>>, StatusCode> {
    state
        .db
        .list_reservations_for_user(user.0.id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[instrument(skip(state))]
pub async fn cancel_reservation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Response {
    match state.db.cancel_reservation(id, user.0.id).await {
        Ok(Some(r)) => Json(r).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Demo endpoint that sleeps in Postgres so SigNoz shows a slow SQL span.
#[instrument(skip(state))]
pub async fn slow_probe(State(state): State<AppState>, _user: AuthUser) -> Response {
    match state.db.slow_probe().await {
        Ok(()) => Json(serde_json::json!({"ok": true, "slept_seconds": 1.5})).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
