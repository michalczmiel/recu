#![allow(clippy::module_name_repetitions)]

use std::io;

use clap::{Args, Subcommand, ValueEnum};
use rusty_money::{Findable, iso};

use crate::config;

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
                let code = args.value.to_uppercase();
                if iso::Currency::find(&code).is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown currency code: {code}"),
                    ));
                }
                let mut cfg = config::load()?;
                cfg.currency = Some(code.clone());
                config::save(&cfg)?;
                println!("currency set to {code}");
            }
        },
    }
    Ok(())
}
