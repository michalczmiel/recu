use std::io;

use clap::{Args, Subcommand};

use crate::config;

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// List all defined categories
    List,
    /// Add a new category
    Add(CategoryAddArgs),
    /// Remove a category
    Rm(CategoryRmArgs),
}

#[derive(Args, Debug)]
pub struct CategoryAddArgs {
    /// Category name to add
    pub name: String,
}

#[derive(Args, Debug)]
pub struct CategoryRmArgs {
    /// Category name to remove
    pub name: String,
}

pub fn run(cmd: &CategoryCommand) -> io::Result<()> {
    match cmd {
        CategoryCommand::List => {
            let cfg = config::load()?;
            if cfg.categories.is_empty() {
                println!("No categories defined.");
            } else {
                for cat in &cfg.categories {
                    println!("{cat}");
                }
            }
        }

        CategoryCommand::Add(args) => {
            let mut cfg = config::load()?;
            if cfg
                .categories
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&args.name))
            {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("category '{}' already exists", args.name),
                ));
            }
            cfg.categories.push(args.name.clone());
            cfg.categories.sort();
            config::save(&cfg)?;
            println!("Category '{}' added.", args.name);
        }

        CategoryCommand::Rm(args) => {
            let mut cfg = config::load()?;
            let before = cfg.categories.len();
            cfg.categories
                .retain(|c| !c.eq_ignore_ascii_case(&args.name));
            if cfg.categories.len() == before {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("category '{}' not found", args.name),
                ));
            }
            config::save(&cfg)?;
            println!("Category '{}' removed.", args.name);
        }
    }
    Ok(())
}
