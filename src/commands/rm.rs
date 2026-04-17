use clap::Args;

use crate::store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu rm Netflix
  recu rm netflix              (case-insensitive)
  recu rm @2                   (run 'recu ls' first to see indices)
  recu rm @3,@1                (indices resolved before any removal; use 'recu ls' first)
  recu rm Netflix,Spotify      (comma-separated; prefer @id when mixing with index targets)")]
pub struct RmArgs {
    /// Expense(s) to remove: @id or name (case-insensitive), comma-separated.
    /// When using @id, run 'recu ls' first to see current indices.
    /// For multiple targets, prefer @id to avoid ambiguity.
    #[arg(value_delimiter = ',')]
    pub targets: Vec<String>,
}

pub fn execute(args: &RmArgs) -> std::io::Result<()> {
    let targets: Vec<&str> = args.targets.iter().map(String::as_str).collect();
    let names = store::remove(&targets)?;
    for name in names {
        println!("Removed '{name}'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Expense;
    use std::fs;

    fn test_file() -> std::path::PathBuf {
        let file = std::env::temp_dir().join("recu-test-rm").join(format!(
            "{}.csv",
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_file(&file);
        file
    }

    fn seed_expenses(file: &std::path::Path) {
        let expenses = vec![
            ("Netflix", 9.99, "usd"),
            ("Spotify", 5.99, "usd"),
            ("NY Times", 15.99, "eur"),
        ];

        for (name, amount, currency) in expenses {
            let expense = Expense {
                amount: Some(amount),
                currency: Some(currency.to_string()),
                ..Default::default()
            };
            store::save_to(file, name, &expense).expect("seed save should succeed");
        }
    }

    fn names_in(file: &std::path::Path) -> Vec<String> {
        let mut items = store::list_from(file)
            .expect("list should succeed")
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        items.sort();
        items
    }

    #[test]
    fn remove_by_full_name() {
        let file = test_file();
        seed_expenses(&file);
        assert!(store::remove_from(&file, &["Netflix"]).is_ok());
        let remaining = names_in(&file);
        assert!(!remaining.contains(&"Netflix".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn remove_by_id_first_and_last() {
        for (id, index) in [("@1", 0), ("@3", 2)] {
            let file = test_file();
            seed_expenses(&file);

            let entries = store::list_from(&file).expect("list should succeed");
            let target_name = entries[index].0.clone();

            assert!(store::remove_from(&file, &[id]).is_ok());
            let remaining = names_in(&file);
            assert!(!remaining.contains(&target_name));
            assert_eq!(remaining.len(), 2);
        }
    }

    #[test]
    fn remove_nonexistent_returns_error() {
        let file = test_file();
        seed_expenses(&file);
        let result = store::remove_from(&file, &["Hulu"]);
        assert!(result.is_err());
    }

    #[test]
    fn remove_id_out_of_range_returns_error() {
        let file = test_file();
        seed_expenses(&file);
        assert!(store::remove_from(&file, &["@0"]).is_err());
        assert!(store::remove_from(&file, &["@99"]).is_err());
    }

    #[test]
    fn remove_by_name_case_insensitive() {
        let file = test_file();
        seed_expenses(&file);
        assert!(store::remove_from(&file, &["netflix"]).is_ok());
        let remaining = names_in(&file);
        assert!(!remaining.contains(&"Netflix".to_string()));
    }

    #[test]
    fn remove_many_by_name() {
        let file = test_file();
        seed_expenses(&file);
        let names =
            store::remove_from(&file, &["Netflix", "Spotify"]).expect("remove_many should succeed");
        assert_eq!(names, vec!["Netflix", "Spotify"]);
        let remaining = names_in(&file);
        assert_eq!(remaining, vec!["NY Times"]);
    }

    #[test]
    fn remove_many_by_id_out_of_order() {
        let file = test_file();
        seed_expenses(&file);
        // @3=NY Times, @1=Netflix — specifying highest last; internal reverse must still work
        let names = store::remove_from(&file, &["@3", "@1"]).expect("remove_many should succeed");
        assert_eq!(names, vec!["NY Times", "Netflix"]);
        let remaining = names_in(&file);
        assert_eq!(remaining, vec!["Spotify"]);
    }
}
