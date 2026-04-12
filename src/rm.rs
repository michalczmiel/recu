use clap::Args;

use crate::storage;

#[derive(Args, Debug)]
pub struct RmArgs {
    /// Expense to remove: @id, slug, or full name
    pub target: String,
}

pub fn execute(args: &RmArgs) -> std::io::Result<()> {
    let name = storage::remove(&args.target)?;
    println!("Removed '{name}'");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Expense;
    use std::fs;

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("recu-test-rm")
            .join(std::thread::current().name().unwrap_or("test").to_owned());
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_expenses(dir: &std::path::Path) {
        // Create 3 expenses: Netflix, Spotify, NY Times
        let expenses = vec![
            ("Netflix", 9.99, "usd"),
            ("Spotify", 5.99, "usd"),
            ("NY Times", 15.99, "eur"),
        ];

        for (name, amount, currency) in expenses {
            let expense = Expense {
                amount: Some(amount),
                currency: Some(currency.to_string()),
                first_payment_date: None,
                interval: None,
            };
            storage::save_to(dir, name, &expense).unwrap();
        }
    }

    fn names_in(dir: &std::path::Path) -> Vec<String> {
        let mut items = storage::list_from(dir)
            .unwrap()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        items.sort();
        items
    }

    #[test]
    fn remove_by_full_name() {
        let dir = test_dir();
        seed_expenses(&dir);
        assert!(storage::remove_from(&dir, "Netflix").is_ok());
        let remaining = names_in(&dir);
        assert!(!remaining.contains(&"Netflix".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn remove_by_slug() {
        let dir = test_dir();
        seed_expenses(&dir);
        assert!(storage::remove_from(&dir, "ny-times").is_ok());
        let remaining = names_in(&dir);
        assert!(!remaining.contains(&"NY Times".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn remove_by_id_first_and_last() {
        for (id, index) in [("@1", 0), ("@3", 2)] {
            let dir = test_dir();
            seed_expenses(&dir);

            let entries = storage::list_from(&dir).unwrap();
            let target_name = entries[index].0.clone();

            assert!(storage::remove_from(&dir, id).is_ok());
            let remaining = names_in(&dir);
            assert!(!remaining.contains(&target_name));
            assert_eq!(remaining.len(), 2);
        }
    }

    #[test]
    fn remove_nonexistent_returns_error() {
        let dir = test_dir();
        seed_expenses(&dir);
        let result = storage::remove_from(&dir, "Hulu");
        assert!(result.is_err());
    }

    #[test]
    fn remove_id_out_of_range_returns_error() {
        let dir = test_dir();
        seed_expenses(&dir);
        assert!(storage::remove_from(&dir, "@0").is_err());
        assert!(storage::remove_from(&dir, "@99").is_err());
    }

    #[test]
    fn remove_by_name_case_insensitive() {
        let dir = test_dir();
        seed_expenses(&dir);
        assert!(storage::remove_from(&dir, "netflix").is_ok());
        let remaining = names_in(&dir);
        assert!(!remaining.contains(&"Netflix".to_string()));
    }
}
