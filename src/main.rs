use clap::{Parser, Subcommand};

mod add;
mod edit;
mod expense;
mod ls;
mod rm;
mod storage;

#[derive(Parser, Debug)]
#[command(name = "recu", version, about = "Track recurring expenses")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses
    Ls,
    /// Add a recurring expense
    Add(add::AddArgs),
    /// Edit a recurring expense
    Edit(edit::EditArgs),
    /// Remove a recurring expense
    Rm(rm::RmArgs),
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Ls) => ls::execute()?,
        Some(Commands::Add(args)) => add::execute(&args)?,
        Some(Commands::Edit(args)) => edit::execute(args)?,
        Some(Commands::Rm(args)) => rm::execute(&args)?,
    }
    Ok(())
}
