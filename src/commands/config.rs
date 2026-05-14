#![allow(clippy::module_name_repetitions)]

use std::io;

use clap::{Args, Subcommand, ValueEnum};

use crate::config;
use crate::expense::normalize_currency;

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
