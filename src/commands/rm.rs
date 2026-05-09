use clap::Args;

use crate::commands::OutputFormat;
use crate::store::Store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu rm Netflix
  recu rm netflix              (case-insensitive)
  recu rm @2                   (run 'recu list' first to see indices)
  recu rm @3,@1                (indices resolved before any removal; use 'recu list' first)
  recu rm Netflix,Spotify      (comma-separated; prefer @id when mixing with index targets)")]
pub struct RmArgs {
    /// Expense(s) to remove: @id or name (case-insensitive), comma-separated.
    /// When using @id, run 'recu list' first to see current indices.
    /// For multiple targets, prefer @id to avoid ambiguity.
    #[arg(value_delimiter = ',')]
    pub targets: Vec<String>,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

pub fn execute(args: &RmArgs, store: &Store) -> std::io::Result<()> {
    let targets: Vec<&str> = args.targets.iter().map(String::as_str).collect();
    let names = store.remove(&targets)?;
    match args.format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(std::io::stdout(), &names)?;
            println!();
        }
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
    use crate::expense::Expense;
    use std::fs;

    fn make_store(test_name: &str) -> Store {
        let file = std::env::temp_dir()
            .join("recu-test-rm")
            .join(format!("{test_name}.csv"));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(file.with_extension("csv.seq"));
        let _ = fs::remove_file(file.with_extension("csv.undo"));
        Store::at(file)
    }

    fn seed_expenses(store: &Store) {
        for (name, amount, currency) in [
            ("Netflix", 9.99, "usd"),
            ("Spotify", 5.99, "usd"),
            ("NY Times", 15.99, "eur"),
        ] {
            store
                .save(&Expense {
                    name: name.to_string(),
                    amount: Some(amount),
                    currency: Some(currency.to_string()),
                    ..Default::default()
                })
                .expect("seed save should succeed");
        }
    }

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
        let store = make_store("remove-by-full-name");
        seed_expenses(&store);
        assert!(store.remove(&["Netflix"]).is_ok());
        let remaining = names_in(&store);
        assert!(!remaining.contains(&"Netflix".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn remove_by_id_first_and_last() {
        for (id, index) in [("@1", 0), ("@3", 2)] {
            let store = make_store(&format!("remove-by-id-{index}"));
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
        let store = make_store("remove-nonexistent");
        seed_expenses(&store);
        assert!(store.remove(&["Hulu"]).is_err());
    }

    #[test]
    fn remove_id_out_of_range_returns_error() {
        let store = make_store("remove-id-out-of-range");
        seed_expenses(&store);
        assert!(store.remove(&["@0"]).is_err());
        assert!(store.remove(&["@99"]).is_err());
    }

    #[test]
    fn remove_by_name_case_insensitive() {
        let store = make_store("remove-by-name-case");
        seed_expenses(&store);
        assert!(store.remove(&["netflix"]).is_ok());
        let remaining = names_in(&store);
        assert!(!remaining.contains(&"Netflix".to_string()));
    }

    #[test]
    fn remove_many_by_name() {
        let store = make_store("remove-many-by-name");
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
        let store = make_store("remove-many-by-id-out-of-order");
        seed_expenses(&store);
        let names = store
            .remove(&["@3", "@1"])
            .expect("remove_many should succeed");
        assert_eq!(names, vec!["NY Times", "Netflix"]);
        let remaining = names_in(&store);
        assert_eq!(remaining, vec!["Spotify"]);
    }
}
