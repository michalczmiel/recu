use chrono::NaiveDate;
use clap::{Args, ValueEnum};
use inquire::{CustomType, Select, Text, validator::Validation};
use rusty_money::iso;

use crate::expense::{Expense, ExpenseFields, Interval};

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseFields,

    /// Positional args parsed implicitly by format
    pub args: Vec<String>,
}

struct ParsedExpense {
    name: Option<String>,
    expense: Expense,
}

fn parse(add: AddArgs) -> ParsedExpense {
    let implicit = parse_implicit_args(&add.args);

    ParsedExpense {
        name: add.fields.name.or(implicit.name),
        expense: Expense {
            amount: add.fields.amount.or(implicit.expense.amount),
            currency: add
                .fields
                .currency
                .map(|c| c.to_lowercase())
                .or(implicit.expense.currency),
            first_payment_date: add.fields.date.or(implicit.expense.first_payment_date),
            interval: add.fields.interval.or(implicit.expense.interval),
        },
    }
}

fn is_currency(s: &str) -> bool {
    iso::find(&s.to_uppercase()).is_some()
}

fn parse_implicit_args(args: &[String]) -> ParsedExpense {
    let mut expense = Expense::default();
    let mut name_parts: Vec<&str> = Vec::new();

    for arg in args {
        if expense.interval.is_none()
            && let Ok(iv) = Interval::from_str(arg, true)
        {
            expense.interval = Some(iv);
            continue;
        }
        if expense.first_payment_date.is_none()
            && let Ok(d) = NaiveDate::parse_from_str(arg, "%Y-%m-%d")
        {
            expense.first_payment_date = Some(d);
            continue;
        }
        if expense.currency.is_none() && is_currency(arg) {
            expense.currency = Some(arg.to_lowercase());
            continue;
        }
        if expense.amount.is_none()
            && let Ok(val) = arg.replace(',', ".").parse::<f64>()
        {
            expense.amount = Some(val);
            continue;
        }
        name_parts.push(arg);
    }

    ParsedExpense {
        name: if name_parts.is_empty() {
            None
        } else {
            Some(name_parts.join(" "))
        },
        expense,
    }
}

fn inquire_err(e: &inquire::InquireError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, e.to_string())
}

fn prompt_fields(parsed: &ParsedExpense) -> std::io::Result<(String, Expense)> {
    let initial_name = parsed.name.as_deref().unwrap_or("");
    let name = Text::new("Name:")
        .with_initial_value(initial_name)
        .with_validator(|s: &str| {
            if s.trim().is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .map_err(|e| inquire_err(&e))?;

    let mut amount_prompt = CustomType::<f64>::new("Amount:").with_placeholder("e.g. 9.99");
    if let Some(v) = parsed.expense.amount {
        amount_prompt = amount_prompt.with_default(v);
    }
    let amount = amount_prompt
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    let initial_currency = parsed.expense.currency.as_deref().unwrap_or("");
    let currency = Text::new("Currency (ISO 4217):")
        .with_initial_value(initial_currency)
        .with_placeholder("e.g. usd, eur, gbp")
        .with_validator(|s: &str| {
            if s.is_empty() || is_currency(s) {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Not a valid ISO 4217 currency code".into(),
                ))
            }
        })
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase());

    let mut date_prompt =
        CustomType::<NaiveDate>::new("First payment date:").with_placeholder("YYYY-MM-DD");
    if let Some(d) = parsed.expense.first_payment_date {
        date_prompt = date_prompt.with_default(d);
    }
    let first_payment_date = date_prompt
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    let intervals = vec![
        Interval::Weekly,
        Interval::Monthly,
        Interval::Quarterly,
        Interval::Yearly,
    ];
    let starting_cursor = parsed
        .expense
        .interval
        .as_ref()
        .and_then(|iv| intervals.iter().position(|x| x == iv))
        .unwrap_or(0);
    let interval = Select::new("Interval:", intervals)
        .with_starting_cursor(starting_cursor)
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    Ok((
        name,
        Expense {
            amount,
            currency,
            first_payment_date,
            interval,
        },
    ))
}

pub fn execute(add: AddArgs) -> std::io::Result<()> {
    let interactive = !add.args.is_empty();
    let parsed = parse(add);

    let (name, expense) = if interactive {
        prompt_fields(&parsed)?
    } else {
        let name = parsed.name.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "name is required")
        })?;
        (name, parsed.expense)
    };

    let path = crate::storage::save(&name, &expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    fn implicit(strs: &[&str]) -> ParsedExpense {
        let args: Vec<String> = strs.iter().map(|s| s.to_string()).collect();
        parse_implicit_args(&args)
    }

    fn add_args(flags: AddArgs) -> ParsedExpense {
        parse(flags)
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parses_all_fields_implicitly() {
        let p = implicit(&["Netflix", "9.99", "usd", "2024-01-15"]);
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("usd"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn parses_name_and_amount_only() {
        let p = implicit(&["Gym", "50"]);
        assert_eq!(p.name.as_deref(), Some("Gym"));
        assert_eq!(p.expense.amount, Some(50.0));
        assert_eq!(p.expense.currency, None);
        assert_eq!(p.expense.first_payment_date, None);
    }

    #[test]
    fn joins_multi_word_name() {
        let p = implicit(&["NY", "Times", "15.99"]);
        assert_eq!(p.name.as_deref(), Some("NY Times"));
        assert_eq!(p.expense.amount, Some(15.99));
    }

    #[test]
    fn args_order_does_not_matter() {
        let p = implicit(&["2024-06-01", "EUR", "9.99", "Spotify"]);
        assert_eq!(p.name.as_deref(), Some("Spotify"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-06-01")));
    }

    #[test]
    fn currency_is_case_insensitive() {
        let p = implicit(&["Test", "USD"]);
        assert_eq!(p.expense.currency.as_deref(), Some("usd"));

        let p = implicit(&["Test", "eur"]);
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn decimal_amount() {
        let p = implicit(&["Test", "49.99"]);
        assert_eq!(p.expense.amount, Some(49.99));
    }

    #[test]
    fn comma_decimal_separator() {
        let p = implicit(&["Test", "49,99"]);
        assert_eq!(p.expense.amount, Some(49.99));
    }

    #[test]
    fn name_only() {
        let p = implicit(&["Netflix"]);
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, None);
    }

    fn fields(
        name: Option<&str>,
        amount: Option<f64>,
        currency: Option<&str>,
        date: Option<NaiveDate>,
        interval: Option<Interval>,
    ) -> ExpenseFields {
        ExpenseFields {
            name: name.map(Into::into),
            amount,
            currency: currency.map(Into::into),
            date,
            interval,
        }
    }

    #[test]
    fn flags_override_implicit_args() {
        let p = add_args(AddArgs {
            fields: fields(Some("Override"), None, Some("GBP"), None, None),
            args: vec!["Netflix".into(), "9.99".into(), "usd".into()],
        });
        assert_eq!(p.name.as_deref(), Some("Override"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("gbp"));
    }

    #[test]
    fn flags_only() {
        let p = add_args(AddArgs {
            fields: fields(
                Some("Spotify"),
                Some(9.99),
                Some("EUR"),
                Some(date("2024-01-15")),
                None,
            ),
            args: vec![],
        });
        assert_eq!(p.name.as_deref(), Some("Spotify"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn flags_fill_gaps_in_implicit() {
        let p = add_args(AddArgs {
            fields: fields(None, None, Some("PLN"), None, None),
            args: vec!["Netflix".into(), "9.99".into()],
        });
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("pln"));
    }
}
