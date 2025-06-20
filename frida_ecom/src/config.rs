use serde::Deserialize;
use std::{error::Error, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct ProcessorConfig {
    pub database_url: String,
    pub threads: u32,
    pub sleep_ms: u64,
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct ImporterConfig {
    pub database_url: String,
    pub server_address: String,
    pub log_level: String,
}

impl ProcessorConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

impl ImporterConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}
