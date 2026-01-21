//! Handler for GET /nix-cache-info endpoint.

use http::{Response, StatusCode};

use crate::config::NixCacheGatewayConfig;

/// Handle GET /nix-cache-info request.
///
/// Returns cache metadata in the standard Nix format:
/// ```text
/// StoreDir: /nix/store
/// WantMassQuery: 1
/// Priority: 30
/// ```
pub fn handle_cache_info(config: &NixCacheGatewayConfig) -> Response<String> {
    let body = format!(
        "StoreDir: {}\nWantMassQuery: {}\nPriority: {}\n",
        config.store_dir,
        if config.want_mass_query { 1 } else { 0 },
        config.priority
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-nix-cache-info")
        .header("Content-Length", body.len())
        .body(body)
        .expect("valid response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_info_default() {
        let config = NixCacheGatewayConfig::default();
        let response = handle_cache_info(&config);

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.body();
        assert!(body.contains("StoreDir: /nix/store"));
        assert!(body.contains("WantMassQuery: 1"));
        assert!(body.contains("Priority: 30"));
    }

    #[test]
    fn test_cache_info_custom() {
        let config = NixCacheGatewayConfig {
            store_dir: "/custom/store".to_string(),
            priority: 20,
            want_mass_query: false,
            ..Default::default()
        };
        let response = handle_cache_info(&config);

        let body = response.body();
        assert!(body.contains("StoreDir: /custom/store"));
        assert!(body.contains("WantMassQuery: 0"));
        assert!(body.contains("Priority: 20"));
    }
}
