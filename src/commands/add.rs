use chrono::Local;
use clap::Args;

use crate::expense::{Expense, ExpenseInput};
use crate::prompt::{
    prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval, prompt_name,
    render_config,
};
use crate::store::Store;

/// Add a recurring expense.
///
/// If arguments are omitted, runs interactively.
#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add --name Netflix --amount 9.99 --currency usd --date 2026-05-01 --interval monthly
  recu add          # interactive mode")]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseInput,
}

fn prompt_fields(fields: &ExpenseInput, store: &Store) -> std::io::Result<Expense> {
    let name = prompt_name(fields.name.as_deref().unwrap_or(""))?;
    let amount = prompt_amount(fields.amount)?;
    let currency = prompt_currency(fields.currency.as_deref().unwrap_or(""))?;
    let start_date = prompt_date(fields.date)?;
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
    })
}

pub fn execute(add: &AddArgs, store: &Store) -> std::io::Result<()> {
    let f = &add.fields;
    let expense = if let (Some(name), Some(amount), Some(currency), Some(interval)) =
        (&f.name, f.amount, &f.currency, &f.interval)
    {
        let start_date = Some(f.date.unwrap_or_else(|| Local::now().date_naive()));
        Expense {
            name: name.clone(),
            amount: Some(amount),
            currency: Some(currency.to_lowercase()),
            start_date,
            interval: Some(interval.clone()),
            category: f.category.clone(),
        }
    } else {
        inquire::set_global_render_config(render_config());
        prompt_fields(f, store)?
    };

    store.save(&expense)?;
    println!("Added {}", expense.summary());
    Ok(())
}
