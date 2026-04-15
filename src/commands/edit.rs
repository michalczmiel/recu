use clap::Args;
use inquire::Select;

use crate::commands::prompt::{
    inquire_err, prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval,
    prompt_name_skippable, render_config, save_new_category,
};
use crate::config;
use crate::expense::{Expense, ExpenseInput};
use crate::store;

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
pub struct EditArgs {
    /// Expense to edit: @id or name (case-insensitive)
    pub target: String,
    #[command(flatten)]
    pub fields: ExpenseInput,
}

fn find_current(target: &str) -> std::io::Result<(String, Expense)> {
    let all = store::list()?;
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid id"))?;
        if id == 0 || id > all.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no expense at @{id}"),
            ));
        }
        return Ok(all.into_iter().nth(id - 1).expect("bounds checked above"));
    }
    all.into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(target))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("expense '{target}' not found"),
            )
        })
}

fn menu_items(name: &str, e: &Expense) -> Vec<MenuItem> {
    let d = "—";
    let item = |field, label: &str, val: &str| MenuItem {
        field,
        display: format!("{label:<14} {val}"),
    };
    vec![
        item(Field::Name, "Name", name),
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
            "Next due",
            &e.next_due.map_or_else(|| d.to_string(), |d| d.to_string()),
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

fn prompt_fields(
    current_name: &str,
    current: &Expense,
) -> std::io::Result<(Option<String>, Expense)> {
    let mut working_name = current_name.to_string();
    let mut working = Expense {
        amount: current.amount,
        currency: current.currency.clone(),
        next_due: current.next_due,
        interval: current.interval.clone(),
        category: current.category.clone(),
    };

    loop {
        let choice = Select::new("Edit:", menu_items(&working_name, &working))
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))?;

        match choice {
            None => break,
            Some(item) => match item.field {
                Field::Done => break,
                Field::Name => {
                    if let Some(new) = prompt_name_skippable(&working_name)? {
                        working_name = new;
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
                    if let Some(d) = prompt_date(working.next_due)? {
                        working.next_due = Some(d);
                    }
                }
                Field::Interval => {
                    if let Some(iv) = prompt_interval(working.interval.as_ref())? {
                        working.interval = Some(iv);
                    }
                }
                Field::Category => {
                    let cfg = config::load()?;
                    if let Some(cat) =
                        prompt_category(&cfg.categories, working.category.as_deref())?
                    {
                        working.category = Some(cat);
                    }
                }
            },
        }
    }

    let new_name = if working_name == current_name {
        None
    } else {
        Some(working_name)
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

pub fn execute(args: &EditArgs) -> std::io::Result<()> {
    if has_any_field(&args.fields) {
        let f = &args.fields;
        let patch = Expense {
            amount: f.amount,
            currency: f.currency.as_ref().map(|c| c.to_lowercase()),
            next_due: f.date,
            interval: f.interval.clone(),
            category: f.category.clone(),
        };
        store::update(&args.target, f.name.as_deref(), &patch)?;
    } else {
        inquire::set_global_render_config(render_config());
        let (current_name, current_expense) = find_current(&args.target)?;
        let (new_name, patch) = prompt_fields(&current_name, &current_expense)?;

        if let Some(ref cat) = patch.category {
            save_new_category(cat)?;
        }

        store::update(&args.target, new_name.as_deref(), &patch)?;
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
        // insertion order: Netflix=@1, Spotify=@2, NY Times=@3
        store::update_from(
            &file,
            "@1",
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
