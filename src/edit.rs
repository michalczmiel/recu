use clap::Args;

use crate::expense::ExpenseFields;
use crate::storage::{self, Expense};

#[derive(Args, Debug)]
pub struct EditArgs {
    /// Expense to edit: @id, slug, or full name
    pub target: String,
    #[command(flatten)]
    pub fields: ExpenseFields,
}

pub fn execute(args: EditArgs) -> std::io::Result<()> {
    let patch = Expense {
        amount: args.fields.amount,
        currency: args.fields.currency.map(|c| c.to_lowercase()),
        tags: args.fields.tags,
        first_payment_date: args.fields.date,
        interval: args.fields.interval,
    };
    storage::update(&args.target, args.fields.name.as_deref(), &patch)?;
    println!("Updated '{}'", args.target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Expense, Interval};
    use chrono::NaiveDate;
    use std::fs;

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("recu-test-edit").join(
            std::thread::current()
                .name()
                .unwrap_or("test")
                .replace("::", "-"),
        );
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_expenses(dir: &std::path::Path) {
        let expenses = vec![
            ("Netflix", 9.99, "usd"),
            ("Spotify", 5.99, "usd"),
            ("NY Times", 15.99, "eur"),
        ];
        for (name, amount, currency) in expenses {
            let expense = Expense {
                amount: Some(amount),
                currency: Some(currency.to_string()),
                tags: None,
                first_payment_date: None,
                interval: None,
            };
            storage::save_to(dir, name, &expense).unwrap();
        }
    }

    fn load(dir: &std::path::Path, name: &str) -> Expense {
        storage::list_from(dir)
            .unwrap()
            .into_iter()
            .find(|(n, _)| n == name)
            .unwrap()
            .1
    }

    fn empty() -> Expense {
        Expense {
            amount: None,
            currency: None,
            tags: None,
            first_payment_date: None,
            interval: None,
        }
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn edit_amount_by_name() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Netflix",
            None,
            &Expense {
                amount: Some(12.99),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(load(&dir, "Netflix").amount, Some(12.99));
    }

    #[test]
    fn edit_amount_by_slug() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "ny-times",
            None,
            &Expense {
                amount: Some(20.0),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(load(&dir, "NY Times").amount, Some(20.0));
    }

    #[test]
    fn edit_amount_by_id() {
        let dir = test_dir();
        seed_expenses(&dir);
        // sorted lexicographically: NY Times=@1, Netflix=@2, Spotify=@3 (uppercase < lowercase)
        storage::update_from(
            &dir,
            "@2",
            None,
            &Expense {
                amount: Some(11.11),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(load(&dir, "Netflix").amount, Some(11.11));
    }

    #[test]
    fn edit_currency() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Spotify",
            None,
            &Expense {
                currency: Some("eur".into()),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(load(&dir, "Spotify").currency.as_deref(), Some("eur"));
    }

    #[test]
    fn edit_tags_replaces_existing() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Netflix",
            None,
            &Expense {
                tags: Some(vec!["streaming".into()]),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(
            load(&dir, "Netflix").tags,
            Some(vec!["streaming".to_string()])
        );
    }

    #[test]
    fn edit_interval() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Netflix",
            None,
            &Expense {
                interval: Some(Interval::Yearly),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(load(&dir, "Netflix").interval, Some(Interval::Yearly));
    }

    #[test]
    fn edit_date() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Netflix",
            None,
            &Expense {
                first_payment_date: Some(date("2025-01-01")),
                ..empty()
            },
        )
        .unwrap();
        assert_eq!(
            load(&dir, "Netflix").first_payment_date,
            Some(date("2025-01-01"))
        );
    }

    #[test]
    fn edit_multiple_fields_at_once() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(
            &dir,
            "Spotify",
            None,
            &Expense {
                amount: Some(9.99),
                currency: Some("eur".into()),
                tags: Some(vec!["music".into()]),
                ..empty()
            },
        )
        .unwrap();
        let e = load(&dir, "Spotify");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("eur"));
        assert_eq!(e.tags, Some(vec!["music".to_string()]));
    }

    #[test]
    fn edit_name_renames_file() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(&dir, "Netflix", Some("Netflix Plus"), &empty()).unwrap();
        let names: Vec<String> = storage::list_from(&dir)
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(names.contains(&"Netflix Plus".to_string()));
        assert!(!names.contains(&"Netflix".to_string()));
        assert!(!dir.join("netflix.md").exists());
        assert!(dir.join("netflix-plus.md").exists());
    }

    #[test]
    fn edit_name_conflict_returns_error() {
        let dir = test_dir();
        seed_expenses(&dir);
        let result = storage::update_from(&dir, "Netflix", Some("Spotify"), &empty());
        assert!(result.is_err());
    }

    #[test]
    fn edit_nonexistent_returns_error() {
        let dir = test_dir();
        seed_expenses(&dir);
        let result = storage::update_from(
            &dir,
            "Hulu",
            None,
            &Expense {
                amount: Some(5.0),
                ..empty()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn edit_id_out_of_range_returns_error() {
        let dir = test_dir();
        seed_expenses(&dir);
        assert!(
            storage::update_from(
                &dir,
                "@0",
                None,
                &Expense {
                    amount: Some(1.0),
                    ..empty()
                }
            )
            .is_err()
        );
        assert!(
            storage::update_from(
                &dir,
                "@99",
                None,
                &Expense {
                    amount: Some(1.0),
                    ..empty()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn empty_patch_leaves_expense_unchanged() {
        let dir = test_dir();
        seed_expenses(&dir);
        storage::update_from(&dir, "Netflix", None, &empty()).unwrap();
        let e = load(&dir, "Netflix");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
    }
}
