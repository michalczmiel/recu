use clap::{Parser, Subcommand};

mod add;

#[derive(Parser, Debug)]
#[command(name = "recu")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses
    Ls,
    /// Add a recurring expense
    Add(add::AddArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ls => {
            println!("Listing recurring expenses...");
        }
        Commands::Add(args) => add::execute(args),
    }
}
