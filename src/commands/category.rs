use std::io;

use clap::{Args, Subcommand};

use crate::store;

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// List categories currently used by expenses
    List,
    /// Remove a category from all matching expenses
    Rm(CategoryRmArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu category rm streaming")]
pub struct CategoryRmArgs {
    /// Category name to remove
    pub name: String,
}

pub fn run(cmd: &CategoryCommand) -> io::Result<()> {
    match cmd {
        CategoryCommand::List => {
            let categories = store::categories()?;
            if categories.is_empty() {
                println!("No categories found.");
            } else {
                for cat in &categories {
                    println!("{cat}");
                }
            }
        }
        CategoryCommand::Rm(args) => {
            let updated = store::clear_category(&args.name)?;
            if updated == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("category '{}' not found", args.name),
                ));
            }
            println!(
                "Removed category '{}' from {} expense(s).",
                args.name, updated
            );
        }
    }
    Ok(())
}
