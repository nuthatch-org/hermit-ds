//! Hermit Data Service gateway — Horizon TAP v2 (GraphTally) payment layer with
//! per-endpoint pricing.
//!
//! horizon-core provides receipt validation, RAV aggregation, on-chain collection,
//! and the TAP-gated reverse proxy. This binary adds a `PricingPolicy` (see
//! pricing.rs): the minimum receipt value is computed per request path, and
//! underpaid requests are rejected with HTTP 402 before being proxied upstream.

use std::sync::Arc;

use horizon_core::{pricing::FnPricing, Config, SharedPricing};

mod pricing;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermit_gateway=info,horizon_core=info".into()),
        )
        .init();

    let config = Config::load()?;
    let policy: SharedPricing = Arc::new(FnPricing(|path: &str| pricing::min_receipt_value(path)));

    tracing::info!(
        upstream = %config.backend.upstream_url,
        data_service = %config.tap.data_service_address,
        "Hermit Data Service gateway starting — per-endpoint pricing enforced"
    );

    // No custom routes here; pass an empty router. Add routes (e.g. WebSocket) by
    // building a Router<AppState> and threading it through run_with / run_state.
    horizon_core::run_with(config, policy, axum::Router::new()).await
}
