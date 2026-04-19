use clap::Args;
use inquire::Select;

use crate::expense::{Expense, ExpenseInput};
use crate::prompt::{
    inquire_err, prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval,
    prompt_name_skippable, render_config,
};
use crate::store::Store;

#[derive(Clone, PartialEq)]
enum Field {
    Name,
    Amount,
    Currency,
    Date,
    Interval,
    Category,
    Done,
}

struct MenuItem {
    field: Field,
    display: String,
}

impl std::fmt::Display for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu edit @1 -a 12.99
  recu edit Netflix --interval yearly
  recu edit Netflix          # interactive mode")]
pub struct EditArgs {
    /// Expense to edit: @id or name (case-insensitive)
    pub target: String,
    #[command(flatten)]
    pub fields: ExpenseInput,
}

fn menu_items(e: &Expense) -> Vec<MenuItem> {
    let d = "—";
    let item = |field, label: &str, val: &str| MenuItem {
        field,
        display: format!("{label:<14} {val}"),
    };
    vec![
        item(Field::Name, "Name", &e.name),
        item(
            Field::Amount,
            "Amount",
            &e.amount.map_or_else(|| d.to_string(), |a| a.to_string()),
        ),
        item(
            Field::Currency,
            "Currency",
            e.currency.as_deref().unwrap_or(d),
        ),
        item(
            Field::Date,
            "Start date",
            &e.start_date
                .map_or_else(|| d.to_string(), |d| d.to_string()),
        ),
        item(
            Field::Interval,
            "Interval",
            &e.interval
                .as_ref()
                .map_or_else(|| d.to_string(), std::string::ToString::to_string),
        ),
        item(
            Field::Category,
            "Category",
            e.category.as_deref().unwrap_or(d),
        ),
        MenuItem {
            field: Field::Done,
            display: "Done".to_string(),
        },
    ]
}

fn prompt_fields(current: &Expense, store: &Store) -> std::io::Result<(Option<String>, Expense)> {
    let mut working = current.clone();

    loop {
        let choice = Select::new("Edit:", menu_items(&working))
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))?;

        match choice {
            None => break,
            Some(item) => match item.field {
                Field::Done => break,
                Field::Name => {
                    if let Some(new) = prompt_name_skippable(&working.name)? {
                        working.name = new;
                    }
                }
                Field::Amount => {
                    if let Some(v) = prompt_amount(working.amount)? {
                        working.amount = Some(v);
                    }
                }
                Field::Currency => {
                    if let Some(c) = prompt_currency(working.currency.as_deref().unwrap_or(""))? {
                        working.currency = Some(c);
                    }
                }
                Field::Date => {
                    if let Some(d) = prompt_date(working.start_date)? {
                        working.start_date = Some(d);
                    }
                }
                Field::Interval => {
                    if let Some(iv) = prompt_interval(working.interval.as_ref())? {
                        working.interval = Some(iv);
                    }
                }
                Field::Category => {
                    let categories = store.categories()?;
                    if let Some(cat) = prompt_category(&categories, working.category.as_deref())? {
                        working.category = Some(cat);
                    }
                }
            },
        }
    }

    let new_name = if working.name == current.name {
        None
    } else {
        Some(working.name.clone())
    };
    Ok((new_name, working))
}

fn has_any_field(f: &ExpenseInput) -> bool {
    f.name.is_some()
        || f.amount.is_some()
        || f.currency.is_some()
        || f.date.is_some()
        || f.interval.is_some()
        || f.category.is_some()
}

pub fn execute(args: &EditArgs, store: &Store) -> std::io::Result<()> {
    if has_any_field(&args.fields) {
        let f = &args.fields;
        let patch = Expense {
            amount: f.amount,
            currency: f.currency.as_ref().map(|c| c.to_lowercase()),
            start_date: f.date,
            interval: f.interval.clone(),
            category: f.category.clone(),
            ..Default::default()
        };
        store.update(&args.target, f.name.as_deref(), &patch)?;
    } else {
        inquire::set_global_render_config(render_config());
        let current = store.get(&args.target)?;
        let (new_name, patch) = prompt_fields(&current, store)?;
        store.update(&args.target, new_name.as_deref(), &patch)?;
    }
    println!("Updated '{}'", args.target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::{Expense, Interval};
    use chrono::NaiveDate;
    use std::fs;

    fn make_store(test_name: &str) -> Store {
        let file = std::env::temp_dir()
            .join("recu-test-edit")
            .join(format!("{}.csv", test_name.replace("::", "-")));
        let _ = fs::remove_file(&file);
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

    fn load(store: &Store, name: &str) -> Expense {
        store
            .list()
            .expect("list should succeed")
            .into_iter()
            .find(|e| e.name == name)
            .expect("expense should exist")
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid date literal")
    }

    #[test]
    fn edit_amount_by_name() {
        let store = make_store("edit-amount-by-name");
        seed_expenses(&store);
        store
            .update(
                "Netflix",
                None,
                &Expense {
                    amount: Some(12.99),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").amount, Some(12.99));
    }

    #[test]
    fn edit_amount_by_id() {
        let store = make_store("edit-amount-by-id");
        seed_expenses(&store);
        // insertion order: Netflix=@1, Spotify=@2, NY Times=@3
        store
            .update(
                "@1",
                None,
                &Expense {
                    amount: Some(11.11),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").amount, Some(11.11));
    }

    #[test]
    fn edit_currency() {
        let store = make_store("edit-currency");
        seed_expenses(&store);
        store
            .update(
                "Spotify",
                None,
                &Expense {
                    currency: Some("eur".into()),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Spotify").currency.as_deref(), Some("eur"));
    }

    #[test]
    fn edit_interval() {
        let store = make_store("edit-interval");
        seed_expenses(&store);
        store
            .update(
                "Netflix",
                None,
                &Expense {
                    interval: Some(Interval::Yearly),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").interval, Some(Interval::Yearly));
    }

    #[test]
    fn edit_date() {
        let store = make_store("edit-date");
        seed_expenses(&store);
        store
            .update(
                "Netflix",
                None,
                &Expense {
                    start_date: Some(date("2025-01-01")),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").start_date, Some(date("2025-01-01")));
    }

    #[test]
    fn edit_multiple_fields_at_once() {
        let store = make_store("edit-multiple-fields");
        seed_expenses(&store);
        store
            .update(
                "Spotify",
                None,
                &Expense {
                    amount: Some(9.99),
                    currency: Some("eur".into()),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        let e = load(&store, "Spotify");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn edit_name_updates_stored_name() {
        let store = make_store("edit-name-updates");
        seed_expenses(&store);
        store
            .update("Netflix", Some("Netflix Plus"), &Expense::default())
            .expect("update should succeed");
        let names: Vec<String> = store
            .list()
            .expect("list should succeed")
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert!(names.contains(&"Netflix Plus".to_string()));
        assert!(!names.contains(&"Netflix".to_string()));
    }

    #[test]
    fn edit_name_conflict_returns_error() {
        let store = make_store("edit-name-conflict");
        seed_expenses(&store);
        assert!(
            store
                .update("Netflix", Some("Spotify"), &Expense::default())
                .is_err()
        );
    }

    #[test]
    fn edit_nonexistent_returns_error() {
        let store = make_store("edit-nonexistent");
        seed_expenses(&store);
        assert!(
            store
                .update(
                    "Hulu",
                    None,
                    &Expense {
                        amount: Some(5.0),
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn edit_id_out_of_range_returns_error() {
        let store = make_store("edit-id-out-of-range");
        seed_expenses(&store);
        assert!(
            store
                .update(
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
            store
                .update(
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
        let store = make_store("edit-empty-patch");
        seed_expenses(&store);
        store
            .update("Netflix", None, &Expense::default())
            .expect("update should succeed");
        let e = load(&store, "Netflix");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
    }
}
