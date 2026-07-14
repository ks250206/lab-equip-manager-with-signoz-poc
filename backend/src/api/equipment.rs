use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    api::extractors::AuthUser,
    infra::ObjectStore,
    models::{Equipment, Role},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateEquipmentRequest {
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

#[instrument(skip(state))]
pub async fn list_equipment(State(state): State<AppState>) -> Result<Json<Vec<Equipment>>, StatusCode> {
    state
        .db
        .list_equipment()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[instrument(skip(state))]
pub async fn get_equipment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Equipment>, StatusCode> {
    match state.db.get_equipment(id).await {
        Ok(Some(e)) => Ok(Json(e)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[instrument(skip(state, body))]
pub async fn create_equipment(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateEquipmentRequest>,
) -> Response {
    if user.0.role_enum() != Role::Admin {
        return StatusCode::FORBIDDEN.into_response();
    }
    let name = body.name.trim();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "name_required"})),
        )
            .into_response();
    }
    match state
        .db
        .create_equipment(
            name,
            body.description.as_deref().unwrap_or(""),
            body.location.as_deref().unwrap_or(""),
            user.0.id,
            None,
        )
        .await
    {
        Ok(e) => (StatusCode::CREATED, Json(e)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[instrument(skip(state, multipart))]
pub async fn upload_equipment_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Response {
    if user.0.role_enum() != Role::Admin {
        return StatusCode::FORBIDDEN.into_response();
    }

    let Some(equipment) = (match state.db.get_equipment(id).await {
        Ok(e) => e,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut file_bytes: Option<(String, String, Vec<u8>)> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("image") {
            continue;
        }
        let filename = field
            .file_name()
            .unwrap_or("image.bin")
            .to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let Ok(data) = field.bytes().await else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        file_bytes = Some((filename, content_type, data.to_vec()));
        break;
    }

    let Some((filename, content_type, data)) = file_bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "image_required"})),
        )
            .into_response();
    };

    let key = ObjectStore::new_equipment_image_key(equipment.id, &filename);
    if let Err(err) = state.store.put_object(&key, data, &content_type).await {
        tracing::error!(?err, "garage put_object failed");
        return StatusCode::BAD_GATEWAY.into_response();
    }

    match state.db.update_equipment_image(id, &key).await {
        Ok(Some(e)) => Json(e).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
