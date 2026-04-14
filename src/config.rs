#![allow(clippy::module_name_repetitions)]

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub currency: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

fn config_path() -> io::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot determine home directory"))
        .map(|home| home.join(".config").join("recu").join("config"))
}

pub fn load() -> io::Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

pub(crate) fn save(config: &Config) -> io::Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string(config).map_err(|e| io::Error::other(e.to_string()))?;
    write_atomic(&path, &content)
}

fn write_atomic(path: &Path, content: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(tmp, path)
}
