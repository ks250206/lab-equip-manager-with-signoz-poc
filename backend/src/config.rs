use std::env;
use std::net::IpAddr;
use std::str::FromStr;

use ipnet::IpNet;

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
    /// CIDRs treated as trusted reverse proxies for X-Forwarded-For.
    pub trusted_proxies: Vec<IpNet>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://reservations:reservations@localhost:5432/reservations".into()
            }),
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
            garage_secret_key: env::var("GARAGE_SECRET_KEY")
                .unwrap_or_else(|_| "abcdefghijklmnopqrstuvwxyz0123456789ABCDEF".into()),
            garage_bucket: env::var("GARAGE_BUCKET")
                .unwrap_or_else(|_| "equipment-images".into()),
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:14317".into()),
            service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "equipment-reservation-api".into()),
            trusted_proxies: parse_trusted_proxies(
                &env::var("TRUSTED_PROXIES").unwrap_or_else(|_| {
                    // Dedicated Compose proxy network (Caddy → backend only).
                    "172.30.0.0/24".into()
                }),
            )?,
        })
    }

    pub fn is_trusted_proxy(&self, ip: IpAddr) -> bool {
        self.trusted_proxies.iter().any(|net| net.contains(&ip))
    }
}

fn parse_trusted_proxies(raw: &str) -> anyhow::Result<Vec<IpNet>> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| IpNet::from_str(s).map_err(|e| anyhow::anyhow!("invalid TRUSTED_PROXIES entry {s}: {e}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dedicated_proxy_cidr() {
        let nets = parse_trusted_proxies("172.30.0.0/24").unwrap();
        assert_eq!(nets.len(), 1);
        assert!(nets[0].contains(&"172.30.0.2".parse::<IpAddr>().unwrap()));
    }
}
