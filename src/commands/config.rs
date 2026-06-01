#![allow(clippy::module_name_repetitions)]

use std::io;

use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::commands::emit_json;
use crate::config;
use crate::expense::normalize_currency;

#[derive(Serialize)]
struct JsonConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Print current configuration
    List(ConfigListArgs),
    /// Set a configuration value
    Set(ConfigSetArgs),
}

#[derive(Args, Debug)]
pub struct ConfigListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu config set currency usd
  recu config set currency eur")]
pub struct ConfigSetArgs {
    /// Configuration key to set
    #[arg(value_enum)]
    pub key: ConfigKey,
    /// Value to assign
    pub value: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigKey {
    /// Display currency for expense conversion (ISO 4217 code, e.g. USD)
    Currency,
}

pub fn run(cmd: &ConfigCommand) -> io::Result<()> {
    match cmd {
        ConfigCommand::List(args) => {
            let cfg = config::load()?;
            if args.json {
                emit_json(
                    &mut std::io::stdout(),
                    &JsonConfig {
                        currency: cfg.currency.clone(),
                    },
                )?;
            } else {
                match cfg.currency {
                    Some(ref c) => println!("currency = {c}"),
                    None => println!("(no configuration set)"),
                }
            }
        }
        ConfigCommand::Set(args) => match args.key {
            ConfigKey::Currency => {
                let code = normalize_currency(&args.value)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                let mut cfg = config::load()?;
                cfg.currency = Some(code.clone());
                config::save(&cfg)?;
                println!("Currency set to {code}");
            }
        },
    }
    Ok(())
}
