use clap::Args;

use crate::commands::{JsonExpense, OutputFormat, emit_json};
use crate::expense::{Expense, ExpenseFields};
use crate::prompt::{
    install_render_config, pick, prompt_amount, prompt_category, prompt_currency, prompt_date,
    prompt_interval,
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

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu edit @1 -a 12.99
  recu edit Netflix --interval yearly
  recu edit Netflix          # interactive mode")]
pub struct EditArgs {
    /// Expense to edit: @id or name (case-insensitive)
    pub target: String,
    #[command(flatten)]
    pub fields: ExpenseFields,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

fn display<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(|| "—".to_string(), |x| x.to_string())
}

fn menu_items(e: &Expense) -> Vec<MenuItem> {
    let item = |field, label: &str, val: String| MenuItem {
        field,
        display: format!("{label:<14} {val}"),
    };
    vec![
        item(Field::Amount, "Amount", display(e.amount)),
        item(Field::Currency, "Currency", display(e.currency.as_deref())),
        item(Field::Date, "Start date", display(e.start_date)),
        item(Field::Interval, "Interval", display(e.interval.as_ref())),
        item(Field::Category, "Category", display(e.category.as_deref())),
        item(Field::EndDate, "End date", display(e.end_date)),
        MenuItem {
            field: Field::Done,
            display: "Done".to_string(),
        },
    ]
}

fn prompt_fields(current: &Expense, store: &Store) -> std::io::Result<Expense> {
    let mut working = current.clone();

    loop {
        let choice = pick("Edit:", menu_items(&working))?;

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

pub fn execute(args: &EditArgs, store: &Store) -> std::io::Result<()> {
    let patch = if args.fields == ExpenseFields::default() {
        install_render_config();
        let current = store.get(&args.target)?;
        prompt_fields(&current, store)?
    } else {
        Expense::from(&args.fields)
    };
    let updated = store.update(&args.target, &patch)?;
    match args.format {
        OutputFormat::Json => {
            emit_json(&mut std::io::stdout(), &JsonExpense::from(&updated))?;
        }
        OutputFormat::Text => println!("Updated '{}'", args.target),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::{Expense, Interval};
    use crate::test_support;
    use chrono::NaiveDate;

    use test_support::seed_basic as seed_expenses;

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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
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
        let store = test_support::store();
        seed_expenses(&store);
        store
            .update("Netflix", &Expense::default())
            .expect("update should succeed");
        let e = load(&store, "Netflix");
        assert_eq!(e.amount, Some(9.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
    }
}
