use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Role::User),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn role_enum(&self) -> Role {
        Role::parse(&self.role).unwrap_or(Role::User)
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Equipment {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub location: String,
    pub image_object_key: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Reservation {
    pub id: Uuid,
    pub equipment_id: Uuid,
    pub user_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub access_token_hash: Vec<u8>,
    pub refresh_token_hash: Vec<u8>,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}
