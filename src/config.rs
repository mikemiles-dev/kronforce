use std::time::Duration;

pub struct Config {
    pub db_path: String,
    pub bind_addr: String,
    pub tick_interval: Duration,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            db_path: std::env::var("KRONFORCE_DB")
                .unwrap_or_else(|_| "kronforce.db".to_string()),
            bind_addr: std::env::var("KRONFORCE_BIND")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            tick_interval: Duration::from_secs(
                std::env::var("KRONFORCE_TICK_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1),
            ),
        }
    }
}
