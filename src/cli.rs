use clap::{Parser, Subcommand};

use crate::commands::{add, category, config, edit, ls, rm, treemap, undo, upcoming};

#[derive(Parser, Debug)]
#[command(
    name = "recu",
    version,
    about = "Track recurring expenses",
    long_about = "Track recurring expenses. Uses ./recu.csv by default, or RECU_FILE to override the storage file path."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses. Amounts converted to display currency when configured.
    Ls,
    /// Add a recurring expense
    Add(add::AddArgs),
    /// Edit a recurring expense
    Edit(edit::EditArgs),
    /// Remove a recurring expense
    Rm(rm::RmArgs),
    /// Visualize expenses as a treemap
    Treemap,
    /// Manage configuration
    #[command(after_help = "Examples:
  recu config list
  recu config set currency usd")]
    Config {
        #[command(subcommand)]
        command: config::ConfigCommand,
    },
    /// Manage expense categories
    #[command(after_help = "Examples:
  recu category list
  recu category add streaming
  recu category rm streaming")]
    Category {
        #[command(subcommand)]
        command: category::CategoryCommand,
    },
    /// Show upcoming expenses as a timeline. Groups by due date over the next N days.
    Upcoming(upcoming::UpcomingArgs),
    /// Undo the last add, edit, or rm
    Undo,
}

pub fn run() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Ls) => ls::execute()?,
        Some(Commands::Add(args)) => add::execute(&args)?,
        Some(Commands::Edit(args)) => edit::execute(&args)?,
        Some(Commands::Rm(args)) => rm::execute(&args)?,
        Some(Commands::Treemap) => treemap::execute()?,
        Some(Commands::Config { command }) => config::run(&command)?,
        Some(Commands::Category { command }) => category::run(&command)?,
        Some(Commands::Upcoming(args)) => upcoming::execute(&args)?,
        Some(Commands::Undo) => undo::execute()?,
    }
    Ok(())
}
