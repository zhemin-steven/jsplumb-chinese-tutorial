//! Security-related helpers. For now, mainly CORS utilities.
use axum::http::{HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};

pub fn cors_from_allowlist(allowlist: &[String]) -> CorsLayer {
    if allowlist.iter().any(|o| o == "*" || o.eq_ignore_ascii_case("any")) {
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = allowlist
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(origins)
    }
}
