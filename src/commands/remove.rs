use clap::Args;

use crate::commands::emit_json;
use crate::store::Store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu remove Netflix
  recu remove netflix              (case-insensitive)
  recu remove @2                   (run 'recu list' first to see indices)
  recu remove @3,@1                (indices resolved before any removal; use 'recu list' first)
  recu remove Netflix,Spotify      (comma-separated; prefer @id when mixing with index targets)")]
pub struct RemoveArgs {
    /// Expense(s) to remove: @id or name (case-insensitive), comma-separated.
    /// When using @id, run 'recu list' first to see current indices.
    /// For multiple targets, prefer @id to avoid ambiguity.
    #[arg(value_delimiter = ',')]
    pub targets: Vec<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn execute(args: &RemoveArgs, store: &Store) -> std::io::Result<()> {
    let targets: Vec<&str> = args.targets.iter().map(String::as_str).collect();
    let names = store.remove(&targets)?;
    if args.json {
        emit_json(&mut std::io::stdout(), &names)?;
    } else {
        for name in names {
            println!("Removed '{name}'");
        }
    }
    Ok(())
}
