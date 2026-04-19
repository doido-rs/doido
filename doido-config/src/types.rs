use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub view: ViewConfig,
    pub log: LogConfig,
}


#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub bind: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            bind: "127.0.0.1".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://db/development.sqlite3".to_string(),
            pool_size: 5,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ViewConfig {
    pub engine: String,
    pub templates_dir: String,
    pub layout: String,
    pub hot_reload: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            engine: "tera".to_string(),
            templates_dir: "views".to_string(),
            layout: "application".to_string(),
            hot_reload: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}
