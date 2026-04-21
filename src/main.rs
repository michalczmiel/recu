mod cli;
mod commands;
mod config;
mod expense;
mod prompt;
mod rates;
mod store;

use colored::Colorize;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{} {err}", "Error:".red().bold());
        std::process::exit(1);
    }
}
