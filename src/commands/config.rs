#![allow(clippy::module_name_repetitions)]

use std::io;

use clap::{Args, Subcommand};

use crate::config;
use crate::expense::normalize_currency;

pub const VALID_CONFIG_KEYS: &[&str] = &["currency"];

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Print current configuration
    List,
    /// Set a configuration value
    Set(ConfigSetArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu config set currency usd
  recu config set currency eur")]
pub struct ConfigSetArgs {
    /// Configuration key to set
    #[arg(value_parser = parse_config_key)]
    pub key: ConfigKey,
    /// Value to assign
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigKey {
    /// Display currency for expense conversion (ISO 4217 code, e.g. USD)
    Currency,
}

pub fn parse_config_key(s: &str) -> Result<ConfigKey, String> {
    match s.trim().to_lowercase().as_str() {
        "currency" => Ok(ConfigKey::Currency),
        other => Err(format!(
            "invalid config key \"{other}\"; valid: {}\nexample: recu config set currency usd",
            VALID_CONFIG_KEYS.join(", ")
        )),
    }
}

pub fn run(cmd: &ConfigCommand) -> io::Result<()> {
    match cmd {
        ConfigCommand::List => {
            let cfg = config::load()?;
            match cfg.currency {
                Some(ref c) => println!("currency = {c}"),
                None => println!("(no configuration set)"),
            }
        }
        ConfigCommand::Set(args) => match args.key {
            ConfigKey::Currency => {
                let code = normalize_currency(&args.value)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                let mut cfg = config::load()?;
                cfg.currency = Some(code.clone());
                config::save(&cfg)?;
                println!("currency set to {code}");
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_errors() {
        let mut out = String::new();
        out += "=== unknown config key ===\n";
        out += &parse_config_key("foo").expect_err("foo is not a config key");
        insta::assert_snapshot!(out);
    }
}
