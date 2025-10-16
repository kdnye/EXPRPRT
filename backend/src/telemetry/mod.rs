use std::sync::OnceLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static TELEMETRY: OnceLock<()> = OnceLock::new();

pub fn init() {
    TELEMETRY.get_or_init(|| {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    });
}
