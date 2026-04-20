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
/// Only the name is required. Omit all arguments to run interactively.
#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add -n Netflix          # partial — just the name
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
    let expense = if let Some(name) = &f.name {
        let start_date = f
            .date
            .or_else(|| f.interval.as_ref().map(|_| Local::now().date_naive()));
        Expense {
            name: name.clone(),
            amount: f.amount,
            currency: f.currency.as_ref().map(|c| c.to_lowercase()),
            start_date,
            interval: f.interval.clone(),
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
