use clap::Args;

use crate::commands::{JsonExpense, OutputFormat, emit_json};
use crate::expense::{Expense, ExpenseInput};
use crate::prompt::{
    install_render_config, prompt_amount, prompt_category, prompt_currency, prompt_date,
    prompt_interval, prompt_name,
};
use crate::store::Store;

/// Add a recurring expense.
///
/// Pass --name to skip prompts; any missing fields stay unset (fill in later via
/// 'recu edit'). Without --name, runs interactively.
#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu add Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add Netflix             # stored with name only, fill in later via 'recu edit'
  recu add                     # interactive mode")]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseInput,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

fn prompt_fields(input: &ExpenseInput, store: &Store) -> std::io::Result<Expense> {
    let fields = &input.fields;
    let name = prompt_name(input.name.as_deref().unwrap_or(""))?;
    let amount = prompt_amount(fields.amount)?;
    let currency = prompt_currency(fields.currency.as_deref().unwrap_or(""))?;
    let start_date = prompt_date("Start date:", fields.date)?;
    let interval = prompt_interval(fields.interval.as_ref())?;
    let categories = store.categories()?;
    let category = prompt_category(&categories, fields.category.as_deref())?;
    Ok(Expense {
        name,
        amount,
        currency,
        start_date,
        interval,
        category,
        ..Default::default()
    })
}

pub fn execute(args: &AddArgs, store: &Store) -> std::io::Result<()> {
    let f = &args.fields;
    let expense = if let Some(name) = &f.name {
        Expense {
            name: name.clone(),
            ..Expense::from(&f.fields)
        }
    } else {
        install_render_config();
        prompt_fields(f, store)?
    };

    store.save(&expense)?;

    match args.format {
        OutputFormat::Json => {
            let saved = store.get(&expense.name)?;
            emit_json(&mut std::io::stdout(), &JsonExpense::from(&saved))?;
        }
        OutputFormat::Text => println!("Added {}", expense.summary()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::{ExpenseFields, Interval};
    use crate::test_support;
    use crate::test_support::seed_basic as seed_expenses;
    use chrono::NaiveDate;

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

    fn args_with_name(name: &str) -> AddArgs {
        AddArgs {
            fields: ExpenseInput {
                name: Some(name.to_string()),
                ..Default::default()
            },
            format: OutputFormat::Text,
        }
    }

    #[test]
    fn add_by_name_only() {
        let store = test_support::store();
        execute(&args_with_name("Hulu"), &store).expect("add should succeed");
        let e = load(&store, "Hulu");
        assert_eq!(e.name, "Hulu");
        assert_eq!(e.amount, None);
        assert_eq!(e.currency, None);
    }

    #[test]
    fn add_with_full_fields() {
        let store = test_support::store();
        let args = AddArgs {
            fields: ExpenseInput {
                name: Some("Hulu".into()),
                fields: ExpenseFields {
                    amount: Some(12.99),
                    currency: Some("usd".into()),
                    date: Some(date("2026-05-01")),
                    interval: Some(Interval::Monthly),
                    category: Some("streaming".into()),
                    end_date: Some(date("2026-12-31")),
                },
            },
            format: OutputFormat::Text,
        };
        execute(&args, &store).expect("add should succeed");
        let e = load(&store, "Hulu");
        assert_eq!(e.amount, Some(12.99));
        assert_eq!(e.currency.as_deref(), Some("usd"));
        assert_eq!(e.start_date, Some(date("2026-05-01")));
        assert_eq!(e.interval, Some(Interval::Monthly));
        assert_eq!(e.category.as_deref(), Some("streaming"));
        assert_eq!(e.end_date, Some(date("2026-12-31")));
    }

    #[test]
    fn add_assigns_incrementing_id() {
        let store = test_support::store();
        execute(&args_with_name("Hulu"), &store).expect("add should succeed");
        execute(&args_with_name("Disney"), &store).expect("add should succeed");
        assert_eq!(load(&store, "Hulu").id, 1);
        assert_eq!(load(&store, "Disney").id, 2);
    }

    #[test]
    fn add_duplicate_name_returns_error() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(execute(&args_with_name("Netflix"), &store).is_err());
    }

    #[test]
    fn add_duplicate_name_case_insensitive_returns_error() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(execute(&args_with_name("netflix"), &store).is_err());
    }

    #[test]
    fn add_preserves_existing_expenses() {
        let store = test_support::store();
        seed_expenses(&store);
        execute(&args_with_name("Hulu"), &store).expect("add should succeed");
        let names: Vec<String> = store
            .list()
            .expect("list should succeed")
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert!(names.contains(&"Netflix".to_string()));
        assert!(names.contains(&"Spotify".to_string()));
        assert!(names.contains(&"NY Times".to_string()));
        assert!(names.contains(&"Hulu".to_string()));
    }
}
