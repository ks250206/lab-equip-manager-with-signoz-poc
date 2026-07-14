use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRef;

use crate::{
    auth::RateLimiter,
    config::Config,
    infra::{Db, ObjectStore},
};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub store: ObjectStore,
    pub config: Config,
    pub metrics: crate::telemetry::AppMetrics,
    pub login_ip_limiter: Arc<RateLimiter>,
    pub login_account_limiter: Arc<RateLimiter>,
    pub register_ip_limiter: Arc<RateLimiter>,
    pub register_account_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(
        db: Db,
        store: ObjectStore,
        config: Config,
        metrics: crate::telemetry::AppMetrics,
    ) -> Self {
        let window = Duration::from_secs(60);
        let retry = Duration::from_secs(60);
        Self {
            db,
            store,
            config,
            metrics,
            login_ip_limiter: Arc::new(RateLimiter::new(20, window, retry)),
            login_account_limiter: Arc::new(RateLimiter::new(10, window, retry)),
            register_ip_limiter: Arc::new(RateLimiter::new(10, window, retry)),
            register_account_limiter: Arc::new(RateLimiter::new(5, window, retry)),
        }
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
