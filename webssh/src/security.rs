use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};

use crate::config::Config;

pub fn cors_layer(cfg: &Config) -> CorsLayer {
    if cfg.origin_allowlist.is_empty() {
        // same-origin only: don't allow cross-origin requests
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(Any)
    } else {
        let origins = cfg
            .origin_allowlist
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect::<Vec<_>>();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(Any)
    }
}
