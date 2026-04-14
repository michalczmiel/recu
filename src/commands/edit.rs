use clap::Args;

use crate::expense::{Expense, ExpenseInput};
use crate::store;

#[derive(Args, Debug)]
pub struct EditArgs {
    /// Expense to edit: @id or name (case-insensitive)
    pub target: String,
    #[command(flatten)]
    pub fields: ExpenseInput,
}

pub fn execute(args: EditArgs) -> std::io::Result<()> {
    let patch = Expense {
        amount: args.fields.amount,
        currency: args.fields.currency.map(|c| c.to_lowercase()),
        next_due: args.fields.date,
        interval: args.fields.interval,
        category: args.fields.category,
    };
    store::update(&args.target, args.fields.name.as_deref(), &patch)?;
    println!("Updated '{}'", args.target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::{Expense, Interval};
    use chrono::NaiveDate;
    use std::fs;

    fn test_file() -> std::path::PathBuf {
        let file = std::env::temp_dir().join("recu-test-edit").join(format!(
            "{}.csv",
            std::thread::current()
                .name()
                .unwrap_or("test")
                .replace("::", "-"),
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

    fn load(file: &std::path::Path, name: &str) -> Expense {
        store::list_from(file)
            .expect("list should succeed")
            .into_iter()
            .find(|(n, _)| n == name)
            .expect("expense should exist")
            .1
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid date literal")
    }

    #[test]
    fn edit_amount_by_name() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(
            &file,
            "Netflix",
            None,
            &Expense {
                amount: Some(12.99),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        assert_eq!(load(&file, "Netflix").amount, Some(12.99));
    }

    #[test]
    fn edit_amount_by_id() {
        let file = test_file();
        seed_expenses(&file);
        // sorted lexicographically: NY Times=@1, Netflix=@2, Spotify=@3 (uppercase < lowercase)
        store::update_from(
            &file,
            "@2",
            None,
            &Expense {
                amount: Some(11.11),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        assert_eq!(load(&file, "Netflix").amount, Some(11.11));
    }

    #[test]
    fn edit_currency() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(
            &file,
            "Spotify",
            None,
            &Expense {
                currency: Some("eur".into()),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        assert_eq!(load(&file, "Spotify").currency.as_deref(), Some("eur"));
    }

    #[test]
    fn edit_interval() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(
            &file,
            "Netflix",
            None,
            &Expense {
                interval: Some(Interval::Yearly),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        assert_eq!(load(&file, "Netflix").interval, Some(Interval::Yearly));
    }

    #[test]
    fn edit_date() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(
            &file,
            "Netflix",
            None,
            &Expense {
                next_due: Some(date("2025-01-01")),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        assert_eq!(load(&file, "Netflix").next_due, Some(date("2025-01-01")));
    }

    #[test]
    fn edit_multiple_fields_at_once() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(
            &file,
            "Spotify",
            None,
            &Expense {
                amount: Some(9.99),
                currency: Some("eur".into()),
                ..Default::default()
            },
        )
        .expect("update should succeed");
        let e = load(&file, "Spotify");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn edit_name_updates_stored_name() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(&file, "Netflix", Some("Netflix Plus"), &Default::default())
            .expect("update should succeed");
        let names: Vec<String> = store::list_from(&file)
            .expect("list should succeed")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(names.contains(&"Netflix Plus".to_string()));
        assert!(!names.contains(&"Netflix".to_string()));
    }

    #[test]
    fn edit_name_conflict_returns_error() {
        let file = test_file();
        seed_expenses(&file);
        let result = store::update_from(&file, "Netflix", Some("Spotify"), &Default::default());
        assert!(result.is_err());
    }

    #[test]
    fn edit_nonexistent_returns_error() {
        let file = test_file();
        seed_expenses(&file);
        let result = store::update_from(
            &file,
            "Hulu",
            None,
            &Expense {
                amount: Some(5.0),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn edit_id_out_of_range_returns_error() {
        let file = test_file();
        seed_expenses(&file);
        assert!(
            store::update_from(
                &file,
                "@0",
                None,
                &Expense {
                    amount: Some(1.0),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            store::update_from(
                &file,
                "@99",
                None,
                &Expense {
                    amount: Some(1.0),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn empty_patch_leaves_expense_unchanged() {
        let file = test_file();
        seed_expenses(&file);
        store::update_from(&file, "Netflix", None, &Default::default())
            .expect("update should succeed");
        let e = load(&file, "Netflix");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
    }
}
