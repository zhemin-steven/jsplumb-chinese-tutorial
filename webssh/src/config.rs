use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub origin_allowlist: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { port: 8080, origin_allowlist: vec![] }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut cfg = Config::default();
        if let Ok(port) = env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                cfg.port = p;
            }
        }
        if let Ok(list) = env::var("ORIGIN_ALLOWLIST") {
            // comma or space separated
            let items = list
                .split(|c| c == ',' || c == ' ')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>();
            cfg.origin_allowlist = items;
        }
        cfg
    }
}
