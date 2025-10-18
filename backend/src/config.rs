use std::{env, net::SocketAddr};

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub allow_origins: Vec<String>,
    pub addr: SocketAddr,
}

impl Config {
    pub fn from_env() -> Self {
        // load .env file if present
        let _ = dotenvy::dotenv();

        let host = env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("APP_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);
        let allow_origins = env::var("APP_ALLOW_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .expect("invalid APP_HOST or APP_PORT");

        Self {
            host,
            port,
            allow_origins,
            addr,
        }
    }

    pub fn default_rust_log(&self) -> String {
        env::var("RUST_LOG").unwrap_or_else(|_| "info,backend=debug,tower_http=info".to_string())
    }
}
