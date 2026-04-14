mod cli;
mod commands;
mod config;
mod expense;
mod rates;
mod store;

fn main() -> std::io::Result<()> {
    cli::run()
}
