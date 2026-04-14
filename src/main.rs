mod cli;
mod commands;
mod exchange;
mod expense;
mod storage;

fn main() -> std::io::Result<()> {
    cli::run()
}
