use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub password_pepper: Vec<u8>,
    pub cookie_secure: bool,
    pub bind_addr: String,
    pub frontend_origin: String,
    pub garage_endpoint: String,
    pub garage_region: String,
    pub garage_access_key: String,
    pub garage_secret_key: String,
    pub garage_bucket: String,
    pub otel_endpoint: String,
    pub service_name: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://reservations:reservations@localhost:5432/reservations".into()),
            password_pepper: env::var("PASSWORD_PEPPER")
                .unwrap_or_else(|_| "change-me-to-a-long-random-pepper-value".into())
                .into_bytes(),
            cookie_secure: env::var("COOKIE_SECURE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            garage_endpoint: env::var("GARAGE_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:3900".into()),
            garage_region: env::var("GARAGE_REGION").unwrap_or_else(|_| "garage".into()),
            garage_access_key: env::var("GARAGE_ACCESS_KEY")
                .unwrap_or_else(|_| "GK1234567890abcdef".into()),
            garage_secret_key: env::var("GARAGE_SECRET_KEY").unwrap_or_else(|_| {
                "abcdefghijklmnopqrstuvwxyz0123456789ABCDEF".into()
            }),
            garage_bucket: env::var("GARAGE_BUCKET")
                .unwrap_or_else(|_| "equipment-images".into()),
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:14317".into()),
            service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "equipment-reservation-api".into()),
        })
    }
}
