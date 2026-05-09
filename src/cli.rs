use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{add, calendar, category, config, edit, list, rename, rm, treemap, undo};
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

    /// Include ended expenses (only used when no subcommand is given; equivalent to `recu list --all`)
    #[arg(short, long)]
    all: bool,

    /// Output format (only used when no subcommand is given; equivalent to `recu list --format <FORMAT>`)
    #[arg(long, value_enum)]
    format: Option<list::OutputFormat>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses. Amounts converted to display currency when configured.
    List(list::ListArgs),
    /// Add a recurring expense
    Add(add::AddArgs),
    /// Edit a recurring expense
    Edit(edit::EditArgs),
    /// Rename a recurring expense
    Rename(rename::RenameArgs),
    /// Remove one or more recurring expenses
    Rm(rm::RmArgs),
    /// Visualize expenses as a treemap
    Treemap(treemap::TreemapArgs),
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
    /// Show recurring expenses on a month grid
    Calendar(calendar::CalendarArgs),
    /// Undo the last add, edit, rename, or rm
    Undo,
}

pub fn run() -> std::io::Result<()> {
    let cli = Cli::parse();
    let store = Store::at(cli.file);
    match cli.command.unwrap_or(Commands::List(list::ListArgs {
        all: cli.all,
        format: cli.format.unwrap_or_default(),
        ..Default::default()
    })) {
        Commands::List(args) => list::execute(&args, &store)?,
        Commands::Add(args) => add::execute(&args, &store)?,
        Commands::Edit(args) => edit::execute(&args, &store)?,
        Commands::Rename(args) => rename::execute(&args, &store)?,
        Commands::Rm(args) => rm::execute(&args, &store)?,
        Commands::Treemap(args) => treemap::execute(&args, &store)?,
        Commands::Config { command } => config::run(&command)?,
        Commands::Category { command } => category::run(&command, &store)?,
        Commands::Calendar(args) => calendar::execute(&args, &store)?,
        Commands::Undo => undo::execute(&store)?,
    }
    Ok(())
}
