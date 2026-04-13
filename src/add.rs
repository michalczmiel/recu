use clap::Args;
use inquire::{
    Autocomplete, CustomType, DateSelect, Select, Text,
    ui::{Color, RenderConfig, Styled},
    validator::Validation,
};
use rusty_money::iso;

use crate::expense::{Expense, ExpenseFields, Interval};

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseFields,
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

fn detect_locale_currency() -> Option<String> {
    for var in &["LC_MONETARY", "LC_ALL", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let locale = val.split('.').next()?;
            let country = locale.split('_').nth(1)?;
            if let Some(currency) = country_to_currency(country) {
                return Some(currency.to_string());
            }
        }
    }
    None
}

fn country_to_currency(country: &str) -> Option<&'static str> {
    match country {
        "US" | "EC" | "SV" | "PA" => Some("usd"),
        "GB" => Some("gbp"),
        "AU" => Some("aud"),
        "CA" => Some("cad"),
        "CH" => Some("chf"),
        "CN" => Some("cny"),
        "JP" => Some("jpy"),
        "KR" => Some("krw"),
        "SE" => Some("sek"),
        "NO" => Some("nok"),
        "DK" => Some("dkk"),
        "NZ" => Some("nzd"),
        "SG" => Some("sgd"),
        "HK" => Some("hkd"),
        "IN" => Some("inr"),
        "BR" => Some("brl"),
        "MX" => Some("mxn"),
        "RU" => Some("rub"),
        "ZA" => Some("zar"),
        "PL" => Some("pln"),
        "CZ" => Some("czk"),
        "HU" => Some("huf"),
        "RO" => Some("ron"),
        "TR" => Some("try"),
        "IL" => Some("ils"),
        "TH" => Some("thb"),
        "MY" => Some("myr"),
        "ID" => Some("idr"),
        "PH" => Some("php"),
        "UA" => Some("uah"),
        "AR" => Some("ars"),
        "CL" => Some("clp"),
        "CO" => Some("cop"),
        "PE" => Some("pen"),
        "VN" => Some("vnd"),
        "BD" => Some("bdt"),
        "PK" => Some("pkr"),
        "EG" => Some("egp"),
        "NG" => Some("ngn"),
        "KE" => Some("kes"),
        // Eurozone
        "DE" | "FR" | "IT" | "ES" | "NL" | "BE" | "AT" | "PT" | "FI" | "IE" | "GR" | "SK"
        | "SI" | "LU" | "CY" | "MT" | "EE" | "LV" | "LT" => Some("eur"),
        _ => None,
    }
}

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

fn prompt_fields(fields: &ExpenseFields) -> std::io::Result<(String, Expense)> {
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

    let locale_currency = detect_locale_currency();
    let initial_currency = fields
        .currency
        .as_deref()
        .or(locale_currency.as_deref())
        .unwrap_or("");
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

    let mut date_prompt = DateSelect::new("First payment date:");
    if let Some(d) = fields.date {
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
    let starting_cursor = fields
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

pub fn execute(add: &AddArgs) -> std::io::Result<()> {
    inquire::set_global_render_config(render_config());
    let (name, expense) = prompt_fields(&add.fields)?;
    let path = crate::storage::save(&name, &expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}
