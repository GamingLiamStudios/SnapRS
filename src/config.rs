use std::io::Write;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct LogLevel {
    level: log::LevelFilter,
}

impl From<log::LevelFilter> for LogLevel {
    fn from(level: log::LevelFilter) -> Self {
        Self { level }
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        level.level
    }
}

impl Serialize for LogLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.level.to_string().to_lowercase())
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let level = match s.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Off,
        };
        Ok(Self { level })
    }
}

// Load config from file and merge with default config
lazy_static! {
    pub static ref CONFIG: Config = Config::load("config.toml");
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub network: NetworkConfig,
    pub server: ServerConfig,

    // Don't serialize
    #[serde(skip)]
    path: String,
}

#[derive(Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: LogLevel,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkConfig {
    pub port: u16,
    pub max_players: usize,

    pub advanced: AdvancedNetworkConfig,
}

#[derive(Serialize, Deserialize)]
pub struct AdvancedNetworkConfig {
    pub buffer_size: usize,
    pub buffered_packets: usize,

    pub compression_threshold: u32,
    pub compression_level: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub motd: String,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let default: toml::Value =
            toml::from_str(std::include_str!("../config.default.toml")).unwrap();

        // Create config file if it doesn't exist
        if !std::path::Path::new(path).exists() {
            let mut file = std::fs::File::create(path).unwrap();
            file.write_all(toml::to_string(&default).unwrap().as_bytes())
                .unwrap();
        }

        // Read config from file
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut file, &mut contents).unwrap();

        // Merge default config with config from file
        let cfg_file = toml::from_str(&contents).unwrap();
        let mut cfg = serde_toml_merge::merge(default, cfg_file)
            .unwrap()
            .try_into::<Config>()
            .unwrap();
        cfg.path = path.to_string();
        cfg
    }

    pub fn destroy(&self) {
        // Save config to file
        let mut file = std::fs::File::create(self.path.as_str()).unwrap();
        file.write_all(toml::to_string(&self).unwrap().as_bytes())
            .unwrap();
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        self.destroy();
    }
}
