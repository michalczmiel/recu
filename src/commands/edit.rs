use chrono::NaiveDate;
use clap::Args;
use inquire::Select;

use crate::expense::{Expense, Interval, normalize_currency, parse_amount};
use crate::prompt::{
    inquire_err, prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval,
    render_config,
};
use crate::store::Store;

#[derive(Clone, PartialEq)]
enum Field {
    Amount,
    Currency,
    Date,
    Interval,
    Category,
    EndDate,
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

#[derive(Args, Debug, Default)]
pub struct EditFields {
    /// Amount (e.g. 9.99 or 9,99)
    #[arg(short, long, value_parser = parse_amount)]
    pub amount: Option<f64>,
    /// ISO 4217 currency code (e.g. usd, eur)
    #[arg(short, long, value_parser = normalize_currency)]
    pub currency: Option<String>,
    /// Start date (YYYY-MM-DD)
    #[arg(short, long)]
    pub date: Option<NaiveDate>,
    /// Billing interval
    #[arg(short, long)]
    pub interval: Option<Interval>,
    /// Category label (e.g. streaming, utilities)
    #[arg(long = "category")]
    pub category: Option<String>,
    /// End date — when the subscription stops (YYYY-MM-DD)
    #[arg(long = "end")]
    pub end_date: Option<NaiveDate>,
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
    pub fields: EditFields,
}

fn menu_items(e: &Expense) -> Vec<MenuItem> {
    let d = "—";
    let item = |field, label: &str, val: &str| MenuItem {
        field,
        display: format!("{label:<14} {val}"),
    };
    vec![
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
        item(
            Field::EndDate,
            "End date",
            &e.end_date.map_or_else(|| d.to_string(), |d| d.to_string()),
        ),
        MenuItem {
            field: Field::Done,
            display: "Done".to_string(),
        },
    ]
}

fn prompt_fields(current: &Expense, store: &Store) -> std::io::Result<Expense> {
    let mut working = current.clone();

    loop {
        let choice = Select::new("Edit:", menu_items(&working))
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))?;

        match choice {
            None => break,
            Some(item) => match item.field {
                Field::Done => break,
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
                    if let Some(d) = prompt_date("Start date:", working.start_date)? {
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
                Field::EndDate => {
                    if let Some(d) = prompt_date("End date:", working.end_date)? {
                        working.end_date = Some(d);
                    }
                }
            },
        }
    }

    Ok(working)
}

fn has_any_field(f: &EditFields) -> bool {
    f.amount.is_some()
        || f.currency.is_some()
        || f.date.is_some()
        || f.interval.is_some()
        || f.category.is_some()
        || f.end_date.is_some()
}

pub fn execute(args: &EditArgs, store: &Store) -> std::io::Result<()> {
    if has_any_field(&args.fields) {
        let f = &args.fields;
        let patch = Expense {
            amount: f.amount,
            currency: f.currency.clone(),
            start_date: f.date,
            interval: f.interval.clone(),
            category: f.category.clone(),
            end_date: f.end_date,
            ..Default::default()
        };
        store.update(&args.target, &patch)?;
    } else {
        inquire::set_global_render_config(render_config());
        let current = store.get(&args.target)?;
        let patch = prompt_fields(&current, store)?;
        store.update(&args.target, &patch)?;
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
                &Expense {
                    interval: Some(Interval::Yearly),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").interval, Some(Interval::Yearly));
    }

    #[test]
    fn edit_end_date() {
        let store = make_store("edit-end-date");
        seed_expenses(&store);
        store
            .update(
                "Netflix",
                &Expense {
                    end_date: Some(date("2026-12-31")),
                    ..Default::default()
                },
            )
            .expect("update should succeed");
        assert_eq!(load(&store, "Netflix").end_date, Some(date("2026-12-31")));
    }

    #[test]
    fn edit_date() {
        let store = make_store("edit-date");
        seed_expenses(&store);
        store
            .update(
                "Netflix",
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
    fn edit_nonexistent_returns_error() {
        let store = make_store("edit-nonexistent");
        seed_expenses(&store);
        assert!(
            store
                .update(
                    "Hulu",
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
            .update("Netflix", &Expense::default())
            .expect("update should succeed");
        let e = load(&store, "Netflix");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
    }
}
