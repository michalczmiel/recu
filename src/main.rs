mod cli;
mod commands;
mod config;
mod expense;
mod prompt;
mod rates;
mod store;

fn main() -> std::io::Result<()> {
    cli::run()
}
