use chrono::Local;
use clap::Args;

use crate::commands::prompt::{
    prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval, prompt_name,
    render_config, save_new_category,
};
use crate::config;
use crate::expense::{Expense, ExpenseInput};

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseInput,
}

fn prompt_fields(fields: &ExpenseInput) -> std::io::Result<(String, Expense)> {
    let name = prompt_name(fields.name.as_deref().unwrap_or(""))?;
    let amount = prompt_amount(fields.amount)?;
    let currency = prompt_currency(fields.currency.as_deref().unwrap_or(""))?;
    let next_due = prompt_date(fields.date)?;
    let interval = prompt_interval(fields.interval.as_ref())?;
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
        save_new_category(cat)?;
    }

    let path = crate::store::save(&name, &expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}
