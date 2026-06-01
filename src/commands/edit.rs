use clap::Args;

use crate::commands::{JsonExpense, emit_json};
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
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
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

fn prompt_fields(current: &Expense, name: &str, store: &Store) -> std::io::Result<Expense> {
    let mut working = current.clone();

    loop {
        let choice = pick(&format!("Edit '{name}':"), menu_items(&working))?;

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
        prompt_fields(&current, &current.name, store)?
    } else {
        Expense::from(&args.fields)
    };

    let updated = store.update(&args.target, &patch)?;
    if args.json {
        emit_json(&mut std::io::stdout(), &JsonExpense::from(&updated))?;
    } else {
        println!("Updated '{}'", updated.name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Expense;
    use crate::test_support;
    use chrono::NaiveDate;

    use test_support::seed_basic as seed_expenses;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid date literal")
    }

    #[test]
    fn edit_rejects_end_before_start() {
        let store = test_support::store();
        seed_expenses(&store);
        // First set a start_date on Netflix
        store
            .update(
                "Netflix",
                &Expense {
                    start_date: Some(date("2026-06-01")),
                    ..Default::default()
                },
            )
            .expect("setup should succeed");
        // Now try to set end_date before start_date
        let args = EditArgs {
            target: "Netflix".to_string(),
            fields: ExpenseFields {
                end_date: Some(date("2025-01-01")),
                ..Default::default()
            },
            json: false,
        };
        let err = execute(&args, &store).expect_err("should reject end < start");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
