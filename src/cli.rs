use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{add, category, config, edit, ls, rm, timeline, treemap, undo};
use crate::store::Store;

#[derive(Parser, Debug)]
#[command(name = "recu", version, about = "Track recurring expenses")]
struct Cli {
    /// Path to the CSV storage file
    #[arg(
        short,
        long,
        env = "RECU_FILE",
        default_value = "recu.csv",
        global = true
    )]
    file: PathBuf,

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
    /// Remove one or more recurring expenses
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
  recu category rm streaming
  recu category rename streaming Streaming
  recu category rename streaming,subs Streaming")]
    Category {
        #[command(subcommand)]
        command: category::CategoryCommand,
    },
    /// Show expenses as a timeline. Supports past and future date ranges.
    Timeline(timeline::TimelineArgs),
    /// Undo the last add, edit, or rm
    Undo,
}

pub fn run() -> std::io::Result<()> {
    let cli = Cli::parse();
    let store = Store::at(cli.file);
    match cli.command.unwrap_or(Commands::Ls) {
        Commands::Ls => ls::execute(&store)?,
        Commands::Add(args) => add::execute(&args, &store)?,
        Commands::Edit(args) => edit::execute(&args, &store)?,
        Commands::Rm(args) => rm::execute(&args, &store)?,
        Commands::Treemap => treemap::execute(&store)?,
        Commands::Config { command } => config::run(&command)?,
        Commands::Category { command } => category::run(&command, &store)?,
        Commands::Timeline(args) => timeline::execute(&args, &store)?,
        Commands::Undo => undo::execute(&store)?,
    }
    Ok(())
}
