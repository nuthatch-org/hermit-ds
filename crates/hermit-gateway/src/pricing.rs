//! Per-endpoint compute-unit (CU) pricing for Hermit Data Service.
//!
//! The minimum TAP receipt value for a request path is `cu_cost(path) * BASE_PRICE_PER_CU`.
//! horizon-core enforces it on every request via the `PricingPolicy` wired in main.rs —
//! an underpaid receipt is rejected with HTTP 402.
//!
//! EDIT `cu_cost` to match your service's endpoints and their relative expense.

/// GRT wei per compute unit.
pub const BASE_PRICE_PER_CU: u128 = 4000000000000;

/// Compute-unit cost for a request path. Unknown paths default to the STANDARD tier.
///
/// The tiers below are a starting point — replace them with your real endpoints.
pub fn cu_cost(path: &str) -> u32 {
    let p = path.trim_end_matches('/');

    // BASIC (1 CU) — cheap lookups.
    if p.ends_with("/status") || p.starts_with("/v1/block/") || p.starts_with("/v1/tx/") {
        return 1;
    }
    // AGGREGATE (10 CU) — time-bucketed / large scans.
    if p.starts_with("/v1/gas/") || p.ends_with("/activity") || p.ends_with("/volume") {
        return 10;
    }
    // SQL (20 CU) — arbitrary queries.
    if p.ends_with("/sql") {
        return 20;
    }
    // STANDARD (5 CU) — default.
    5
}

/// Minimum GRT wei required to serve `path`.
pub fn min_receipt_value(path: &str) -> u128 {
    cu_cost(path) as u128 * BASE_PRICE_PER_CU
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers() {
        assert_eq!(cu_cost("/v1/status"), 1);
        assert_eq!(cu_cost("/v1/transfers"), 5);
        assert_eq!(cu_cost("/v1/gas/blocks"), 10);
        assert_eq!(cu_cost("/v1/sql"), 20);
    }

    #[test]
    fn min_value_scales() {
        assert_eq!(min_receipt_value("/v1/status"), BASE_PRICE_PER_CU);
        assert_eq!(min_receipt_value("/v1/sql"), 20 * BASE_PRICE_PER_CU);
    }
}
