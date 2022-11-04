use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub(crate) struct LogLevel {
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

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) general: GeneralConfig,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GeneralConfig {
    pub(crate) log_level: LogLevel,
}

impl Config {
    pub(crate) fn load(path: &str) -> Self {
        let default: toml::Value =
            toml::from_str(std::include_str!("../config.default.toml")).unwrap();

        // Read config from file
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut file, &mut contents).unwrap();

        // Merge default config with config from file
        let config: toml::Value = toml::from_str(&contents).unwrap();
        serde_toml_merge::merge(default, config)
            .unwrap()
            .try_into::<Config>()
            .unwrap()
    }
}
