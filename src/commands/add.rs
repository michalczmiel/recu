use chrono::Local;
use clap::Args;
use inquire::{
    Autocomplete, CustomType, DateSelect, Select, Text,
    ui::{Color, RenderConfig, Styled},
    validator::Validation,
};
use rusty_money::iso;

use crate::config;
use crate::expense::{Expense, ExpenseInput, Interval};

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseInput,
}

fn is_currency(s: &str) -> bool {
    iso::find(&s.to_uppercase()).is_some()
}

fn inquire_err(e: &inquire::InquireError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, e.to_string())
}

fn render_config() -> RenderConfig<'static> {
    RenderConfig::default_colored()
        .with_prompt_prefix(Styled::new("›").with_fg(Color::LightCyan))
        .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
}

// Common ISO 4217 currency codes for autocomplete
const CURRENCIES: &[&str] = &[
    "ars", "aud", "bdt", "brl", "cad", "chf", "clp", "cny", "cop", "czk", "dkk", "egp", "eur",
    "gbp", "hkd", "huf", "idr", "ils", "inr", "jpy", "kes", "krw", "mxn", "myr", "ngn", "nok",
    "nzd", "pen", "php", "pkr", "pln", "ron", "rub", "sek", "sgd", "thb", "try", "uah", "usd",
    "vnd", "zar",
];

#[derive(Clone)]
struct CurrencyCompleter;

impl Autocomplete for CurrencyCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        let input_lower = input.to_lowercase();
        Ok(CURRENCIES
            .iter()
            .filter(|c| c.starts_with(input_lower.as_str()))
            .map(|c| (*c).to_string())
            .collect())
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(highlighted_suggestion)
    }
}

const NEW_CATEGORY_SENTINEL: &str = "+ New category...";

fn prompt_category(
    existing: &[String],
    preselected: Option<&str>,
) -> std::io::Result<Option<String>> {
    if existing.is_empty() {
        return Text::new("Category:")
            .with_initial_value(preselected.unwrap_or(""))
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))
            .map(|opt| opt.filter(|s| !s.is_empty()));
    }

    let mut options: Vec<String> = existing.to_vec();
    options.push(NEW_CATEGORY_SENTINEL.to_string());

    let cursor = preselected
        .and_then(|p| options.iter().position(|o| o == p))
        .unwrap_or(0);

    let chosen = Select::new("Category:", options)
        .with_starting_cursor(cursor)
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    match chosen.as_deref() {
        None => Ok(None),
        Some(s) if s == NEW_CATEGORY_SENTINEL => Text::new("New category name:")
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))
            .map(|opt| opt.filter(|s| !s.is_empty())),
        Some(_) => Ok(chosen),
    }
}

fn prompt_fields(fields: &ExpenseInput) -> std::io::Result<(String, Expense)> {
    let initial_name = fields.name.as_deref().unwrap_or("");
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

    let amount_parser =
        |input: &str| -> Result<f64, ()> { input.replace(',', ".").parse::<f64>().map_err(|_| ()) };
    let mut amount_prompt = CustomType::<f64>::new("Amount:")
        .with_placeholder("e.g. 9.99 or 9,99")
        .with_parser(&amount_parser);
    if let Some(v) = fields.amount {
        amount_prompt = amount_prompt.with_default(v);
    }
    let amount = amount_prompt
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    let initial_currency = fields.currency.as_deref().unwrap_or("");
    let currency = Text::new("Currency (ISO 4217):")
        .with_initial_value(initial_currency)
        .with_placeholder("e.g. usd, eur, gbp")
        .with_autocomplete(CurrencyCompleter)
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

    let mut date_prompt = DateSelect::new("Next due date:");
    if let Some(d) = fields.date {
        date_prompt = date_prompt.with_default(d);
    }
    let next_due = date_prompt
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    let intervals = vec![
        Interval::Weekly,
        Interval::Monthly,
        Interval::Quarterly,
        Interval::Yearly,
    ];
    let starting_cursor = fields
        .interval
        .as_ref()
        .and_then(|iv| intervals.iter().position(|x| x == iv))
        .unwrap_or(0);
    let interval = Select::new("Interval:", intervals)
        .with_starting_cursor(starting_cursor)
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    let cfg = config::load()?;
    let category = prompt_category(&cfg.categories, fields.category.as_deref())?;

    Ok((
        name,
        Expense {
            amount,
            currency,
            next_due,
            interval,
            category,
        },
    ))
}

pub fn execute(add: &AddArgs) -> std::io::Result<()> {
    let f = &add.fields;
    let (name, expense) = if let (Some(name), Some(amount), Some(currency), Some(interval)) =
        (&f.name, f.amount, &f.currency, &f.interval)
    {
        let next_due = Some(f.date.unwrap_or_else(|| Local::now().date_naive()));
        (
            name.clone(),
            Expense {
                amount: Some(amount),
                currency: Some(currency.to_lowercase()),
                next_due,
                interval: Some(interval.clone()),
                category: f.category.clone(),
            },
        )
    } else {
        inquire::set_global_render_config(render_config());
        prompt_fields(f)?
    };

    if let Some(ref cat) = expense.category {
        let mut cfg = config::load()?;
        if !cfg.categories.iter().any(|c| c.eq_ignore_ascii_case(cat)) {
            cfg.categories.push(cat.clone());
            cfg.categories.sort();
            config::save(&cfg)?;
        }
    }

    let path = crate::store::save(&name, &expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}
