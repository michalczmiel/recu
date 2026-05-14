use clap::Args;

use crate::commands::{OutputFormat, emit_json};
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
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

pub fn execute(args: &RemoveArgs, store: &Store) -> std::io::Result<()> {
    let targets: Vec<&str> = args.targets.iter().map(String::as_str).collect();
    let names = store.remove(&targets)?;
    match args.format {
        OutputFormat::Json => emit_json(&mut std::io::stdout(), &names)?,
        OutputFormat::Text => {
            for name in names {
                println!("Removed '{name}'");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;
    use crate::test_support::seed_basic as seed_expenses;

    fn names_in(store: &Store) -> Vec<String> {
        let mut items = store
            .list()
            .expect("list should succeed")
            .into_iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();
        items.sort();
        items
    }

    #[test]
    fn remove_by_full_name() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(store.remove(&["Netflix"]).is_ok());
        let remaining = names_in(&store);
        assert!(!remaining.contains(&"Netflix".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn remove_by_id_first_and_last() {
        for (id, index) in [("@1", 0), ("@3", 2)] {
            let store = test_support::store();
            seed_expenses(&store);
            let target_name = store.list().expect("list should succeed")[index]
                .name
                .clone();
            assert!(store.remove(&[id]).is_ok());
            let remaining = names_in(&store);
            assert!(!remaining.contains(&target_name));
            assert_eq!(remaining.len(), 2);
        }
    }

    #[test]
    fn remove_nonexistent_returns_error() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(store.remove(&["Hulu"]).is_err());
    }

    #[test]
    fn remove_id_out_of_range_returns_error() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(store.remove(&["@0"]).is_err());
        assert!(store.remove(&["@99"]).is_err());
    }

    #[test]
    fn remove_by_name_case_insensitive() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(store.remove(&["netflix"]).is_ok());
        let remaining = names_in(&store);
        assert!(!remaining.contains(&"Netflix".to_string()));
    }

    #[test]
    fn remove_many_by_name() {
        let store = test_support::store();
        seed_expenses(&store);
        let names = store
            .remove(&["Netflix", "Spotify"])
            .expect("remove_many should succeed");
        assert_eq!(names, vec!["Netflix", "Spotify"]);
        let remaining = names_in(&store);
        assert_eq!(remaining, vec!["NY Times"]);
    }

    #[test]
    fn remove_many_by_id_out_of_order() {
        let store = test_support::store();
        seed_expenses(&store);
        let names = store
            .remove(&["@3", "@1"])
            .expect("remove_many should succeed");
        assert_eq!(names, vec!["NY Times", "Netflix"]);
        let remaining = names_in(&store);
        assert_eq!(remaining, vec!["Spotify"]);
    }
}
