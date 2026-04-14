#![allow(clippy::module_name_repetitions)]

use std::io;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use rusty_money::{Findable, iso};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub currency: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Print current configuration
    List,
    /// Set a configuration value
    Set(ConfigSetArgs),
}

#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    /// Configuration key to set
    pub key: ConfigKey,
    /// Value to assign
    pub value: String,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ConfigKey {
    /// Display currency for expense conversion (ISO 4217 code, e.g. USD)
    Currency,
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

fn save(config: &Config) -> io::Result<()> {
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

pub fn run(cmd: &ConfigCommand) -> io::Result<()> {
    match cmd {
        ConfigCommand::List => {
            let config = load()?;
            match config.currency {
                Some(ref c) => println!("currency = {c}"),
                None => println!("(no configuration set)"),
            }
        }
        ConfigCommand::Set(args) => match args.key {
            ConfigKey::Currency => {
                let code = args.value.to_uppercase();
                if iso::Currency::find(&code).is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown currency code: {code}"),
                    ));
                }
                let mut config = load()?;
                config.currency = Some(code.clone());
                save(&config)?;
                println!("currency set to {code}");
            }
        },
    }
    Ok(())
}
