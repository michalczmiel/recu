mod cli;
mod commands;
mod config;
mod expense;
mod prompt;
mod rates;
mod store;
mod ui;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{} {err}", ui::error_label("Error:"));
        std::process::exit(1);
    }
}
