use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    auth::{hash_token, SessionTokens},
    models::{Equipment, Reservation, SessionRow, User},
};

#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, password_hash))]
    pub async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, password_hash, role, created_at
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .fetch_one(&self.pool)
        .await
    }

    #[instrument(skip(self))]
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, role, created_at
            FROM users WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self))]
    pub async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, role, created_at
            FROM users WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self, tokens))]
    pub async fn create_session(
        &self,
        user_id: Uuid,
        tokens: &SessionTokens,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, user_id, access_token_hash, refresh_token_hash,
                access_expires_at, refresh_expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(tokens.access_token_hash.as_slice())
        .bind(tokens.refresh_token_hash.as_slice())
        .bind(tokens.access_expires_at)
        .bind(tokens.refresh_expires_at)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    #[instrument(skip(self, access_token))]
    pub async fn find_session_user_by_access(
        &self,
        access_token: &str,
    ) -> Result<Option<(SessionRow, User)>, sqlx::Error> {
        let hash = hash_token(access_token);
        let session = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, access_token_hash, refresh_token_hash,
                   access_expires_at, refresh_expires_at
            FROM sessions WHERE access_token_hash = $1
            "#,
        )
        .bind(hash.as_slice())
        .fetch_optional(&self.pool)
        .await?;

        let Some(session) = session else {
            return Ok(None);
        };
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, role, created_at FROM users WHERE id = $1
            "#,
        )
        .bind(session.user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user.map(|u| (session, u)))
    }

    pub async fn find_session_by_refresh(
        &self,
        refresh_token: &str,
    ) -> Result<Option<SessionRow>, sqlx::Error> {
        let hash = hash_token(refresh_token);
        sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, user_id, access_token_hash, refresh_token_hash,
                   access_expires_at, refresh_expires_at
            FROM sessions WHERE refresh_token_hash = $1
            "#,
        )
        .bind(hash.as_slice())
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self, tokens))]
    pub async fn rotate_session(
        &self,
        session_id: Uuid,
        previous_refresh_token: &str,
        tokens: &SessionTokens,
    ) -> Result<bool, sqlx::Error> {
        let previous_refresh_token_hash = hash_token(previous_refresh_token);
        let result = sqlx::query(
            r#"
            UPDATE sessions SET
                access_token_hash = $3,
                refresh_token_hash = $4,
                access_expires_at = $5,
                refresh_expires_at = $6,
                updated_at = NOW()
            WHERE id = $1 AND refresh_token_hash = $2
            "#,
        )
        .bind(session_id)
        .bind(previous_refresh_token_hash.as_slice())
        .bind(tokens.access_token_hash.as_slice())
        .bind(tokens.refresh_token_hash.as_slice())
        .bind(tokens.access_expires_at)
        .bind(tokens.refresh_expires_at)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_session_by_refresh(&self, refresh_token: &str) -> Result<(), sqlx::Error> {
        let hash = hash_token(refresh_token);
        sqlx::query("DELETE FROM sessions WHERE refresh_token_hash = $1")
            .bind(hash.as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn list_equipment(&self) -> Result<Vec<Equipment>, sqlx::Error> {
        sqlx::query_as::<_, Equipment>(
            r#"
            SELECT id, name, description, location, image_object_key, created_by, created_at, updated_at
            FROM equipment ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_equipment(&self, id: Uuid) -> Result<Option<Equipment>, sqlx::Error> {
        sqlx::query_as::<_, Equipment>(
            r#"
            SELECT id, name, description, location, image_object_key, created_by, created_at, updated_at
            FROM equipment WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self))]
    pub async fn create_equipment(
        &self,
        name: &str,
        description: &str,
        location: &str,
        created_by: Uuid,
        image_object_key: Option<&str>,
    ) -> Result<Equipment, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Equipment>(
            r#"
            INSERT INTO equipment (id, name, description, location, image_object_key, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, description, location, image_object_key, created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(location)
        .bind(image_object_key)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_equipment_image(
        &self,
        id: Uuid,
        image_object_key: &str,
    ) -> Result<Option<Equipment>, sqlx::Error> {
        sqlx::query_as::<_, Equipment>(
            r#"
            UPDATE equipment SET image_object_key = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, description, location, image_object_key, created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(image_object_key)
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self))]
    pub async fn update_equipment(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        location: Option<&str>,
    ) -> Result<Option<Equipment>, sqlx::Error> {
        sqlx::query_as::<_, Equipment>(
            r#"
            UPDATE equipment SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                location = COALESCE($4, location),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, description, location, image_object_key, created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(location)
        .fetch_optional(&self.pool)
        .await
    }

    #[instrument(skip(self))]
    pub async fn delete_equipment(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM equipment WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    #[instrument(skip(self))]
    pub async fn has_overlapping_reservation(
        &self,
        equipment_id: Uuid,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    ) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM reservations
                WHERE equipment_id = $1
                  AND status = 'active'
                  AND starts_at < $3
                  AND ends_at > $2
            )
            "#,
        )
        .bind(equipment_id)
        .bind(starts_at)
        .bind(ends_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    #[instrument(skip(self))]
    pub async fn create_reservation(
        &self,
        equipment_id: Uuid,
        user_id: Uuid,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    ) -> Result<Reservation, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Reservation>(
            r#"
            INSERT INTO reservations (id, equipment_id, user_id, starts_at, ends_at, status)
            VALUES ($1, $2, $3, $4, $5, 'active')
            RETURNING id, equipment_id, user_id, starts_at, ends_at, status, created_at
            "#,
        )
        .bind(id)
        .bind(equipment_id)
        .bind(user_id)
        .bind(starts_at)
        .bind(ends_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_reservations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Reservation>, sqlx::Error> {
        sqlx::query_as::<_, Reservation>(
            r#"
            SELECT id, equipment_id, user_id, starts_at, ends_at, status, created_at
            FROM reservations WHERE user_id = $1 ORDER BY starts_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn cancel_reservation(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Reservation>, sqlx::Error> {
        sqlx::query_as::<_, Reservation>(
            r#"
            UPDATE reservations SET status = 'cancelled'
            WHERE id = $1 AND user_id = $2 AND status = 'active'
            RETURNING id, equipment_id, user_id, starts_at, ends_at, status, created_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Intentionally slow query for SigNoz latency investigation demos.
    #[instrument(skip(self))]
    pub async fn slow_probe(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT pg_sleep(1.5)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
