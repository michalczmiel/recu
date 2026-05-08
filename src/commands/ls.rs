use std::collections::HashMap;
use std::io::Write;

use chrono::NaiveDate;
use clap::Args;
use colored::Colorize;

use crate::config::{self, Config};
use crate::ui;
use rusty_money::iso;

#[derive(Args, Debug, Default)]
pub struct LsArgs {
    /// Include ended expenses
    #[arg(short, long)]
    pub all: bool,
    /// Filter by category (case-insensitive); comma-separated for multiple
    #[arg(short, long, value_delimiter = ',')]
    pub category: Vec<String>,
}

use crate::expense::{
    self, DueStatus, Expense, RecurringTotals, find_currency, format_amount, format_expense_amount,
};
use crate::rates;
use crate::store::Store;

fn colorize_row(row: &[String], status: &DueStatus) -> Vec<String> {
    row.iter()
        .enumerate()
        .map(|(i, cell)| match i {
            0 => ui::dim(cell).to_string(),
            1 => ui::due(status, &cell.bold().to_string()).to_string(),
            _ => ui::due(status, cell).to_string(),
        })
        .collect()
}

fn build_row(expense: &Expense, today: NaiveDate, show_ends: bool) -> Vec<String> {
    let amount = expense.amount.map_or_else(
        || "-".into(),
        |a| format_expense_amount(expense.currency.as_deref(), a),
    );
    let due_str = if expense.is_ended(today) {
        String::new()
    } else {
        expense
            .next_payment(today)
            .map(|d| ui::format_relative_date(d, today))
            .unwrap_or_default()
    };

    let mut row = vec![
        format!("@{}", expense.id),
        expense.name.clone(),
        amount,
        due_str,
        expense.category.clone().unwrap_or_default(),
    ];
    if show_ends {
        let ends_str = expense
            .end_date
            .map(|d| ui::format_relative_date(d, today))
            .unwrap_or_default();
        row.push(ends_str);
    }
    row
}

fn print_totals(
    out: &mut impl Write,
    totals: &RecurringTotals,
    target_cur: Option<&'static iso::Currency>,
    count: usize,
) -> std::io::Result<()> {
    let Some(cur) = target_cur else { return Ok(()) };
    let label = if count == 1 {
        "1 expense".to_string()
    } else {
        format!("{count} expenses")
    };
    let line = format!(
        "\n{}  {}/month  {}/year",
        label,
        format_amount(cur, totals.monthly),
        format_amount(cur, totals.yearly)
    );
    writeln!(out, "{}", ui::heading(&line))
}

fn print_table(
    out: &mut impl Write,
    rows: &[Vec<String>],
    statuses: &[DueStatus],
    show_ends: bool,
    ended_start: Option<usize>,
) -> std::io::Result<()> {
    let headers: Vec<&str> = if show_ends {
        vec!["@", "name", "amount", "due", "category", "ends"]
    } else {
        vec!["@", "name", "amount", "due", "category"]
    };
    let n = headers.len();
    let widths: Vec<usize> = (0..n)
        .map(|i| {
            rows.iter().fold(ui::char_width(headers[i]), |w, row| {
                w.max(ui::char_width(&row[i]))
            })
        })
        .collect();

    let render_cell = |cell: &str, plain: &str, i: usize| -> String {
        // amount column (index 2) is right-aligned; rest left-aligned
        if i == 2 {
            ui::pad_start(cell, ui::char_width(plain), widths[i])
        } else {
            ui::pad_end(cell, ui::char_width(plain), widths[i])
        }
    };

    let sep_cells: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
    let sep_line = sep_cells.join("  ");

    let header_cells: Vec<String> = (0..n)
        .map(|i| render_cell(headers[i], headers[i], i))
        .collect();
    writeln!(out, "{}", header_cells.join("  "))?;
    writeln!(out, "{sep_line}")?;

    for (idx, (row, status)) in rows.iter().zip(statuses.iter()).enumerate() {
        if Some(idx) == ended_start && idx != 0 {
            writeln!(out, "{sep_line}")?;
        }
        let c = colorize_row(row, status);
        let cells: Vec<String> = (0..n).map(|i| render_cell(&c[i], &row[i], i)).collect();
        writeln!(out, "{}", cells.join("  "))?;
    }
    Ok(())
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[Expense],
    all: bool,
    categories: &[String],
) -> std::io::Result<()> {
    if expenses.is_empty() {
        writeln!(
            out,
            "No expenses yet. Run 'recu add' to track your first subscription."
        )?;
        return Ok(());
    }

    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(find_currency)
        .or_else(|| expense::uniform_currency(expenses));

    let mut visible: Vec<&Expense> = expenses
        .iter()
        .filter(|e| all || !e.is_ended(today))
        .filter(|e| expense::matches_categories(e, categories))
        .collect();
    let hidden_ended = expenses
        .iter()
        .filter(|e| !all && e.is_ended(today) && expense::matches_categories(e, categories))
        .count();

    if visible.is_empty() {
        if categories.is_empty() {
            writeln!(
                out,
                "All {hidden_ended} expenses are ended. Run 'recu ls --all' to view them."
            )?;
        } else {
            writeln!(out, "No expenses match category filter.")?;
        }
        return Ok(());
    }

    let show_ends = visible.iter().any(|e| e.end_date.is_some());

    visible.sort_by_key(|expense| {
        let due = expense.days_until_next(today).unwrap_or(i64::MAX);
        // Ended rows sink to bottom; within each group, sort by next due date.
        (expense.is_ended(today), due)
    });

    let rows: Vec<Vec<String>> = visible
        .iter()
        .map(|expense| build_row(expense, today, show_ends))
        .collect();

    let statuses: Vec<DueStatus> = visible.iter().map(|e| e.due_status(today)).collect();

    let ended_start = visible.iter().position(|e| e.is_ended(today));

    print_table(out, &rows, &statuses, show_ends, ended_start)?;

    let active: Vec<&Expense> = visible
        .iter()
        .copied()
        .filter(|e| !e.is_ended(today))
        .collect();
    let totals = RecurringTotals::compute(active.iter().copied(), exchange_rates.as_ref(), target);
    print_totals(out, &totals, target_cur, active.len())?;

    if hidden_ended > 0 {
        writeln!(
            out,
            "{}",
            ui::dim(&format!("+ {hidden_ended} ended (recu ls --all)"))
        )?;
    }

    Ok(())
}

pub fn execute(args: &LsArgs, store: &Store) -> std::io::Result<()> {
    let expenses = store.list()?;
    let cfg = config::load()?;
    let today = chrono::Local::now().date_naive();
    let categories = crate::commands::category::resolve_filter(&args.category, store)?;
    execute_with(
        &mut std::io::stdout(),
        today,
        &cfg,
        &expenses,
        args.all,
        &categories,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 4, 15).expect("valid date")
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn run(expenses: &[Expense]) -> String {
        run_with(expenses, false)
    }

    fn run_with(expenses: &[Expense], all: bool) -> String {
        let mut buf = Vec::new();
        let with_ids: Vec<Expense> = expenses
            .iter()
            .enumerate()
            .map(|(i, e)| Expense {
                id: (i as u64) + 1,
                ..e.clone()
            })
            .collect();
        execute_with(&mut buf, today(), &Config::default(), &with_ids, all, &[])
            .expect("execute_with");
        String::from_utf8(buf).expect("utf8")
    }

    fn monthly_usd(name: &str, amount: f64, start_date: NaiveDate) -> Expense {
        Expense {
            name: name.to_string(),
            amount: Some(amount),
            currency: Some("usd".to_string()),
            start_date: Some(start_date),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }
    }

    #[test]
    fn ls() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let mut out = String::new();

        out += "=== empty ===\n";
        out += &run(&[]);

        // start_date == today → days = 0 → Overdue
        out += "\n=== single due today ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, today())]);

        // 5 days away → DueSoon
        out += "\n=== single due soon ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 20))]);

        // 77 days away → Distant
        out += "\n=== single distant ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 7, 1))]);

        // Added in reverse order; output sorted by due date, @ids reflect insertion order
        out += "\n=== sorted by due date ===\n";
        out += &run(&[
            monthly_usd("Notion", 16.00, d(2026, 7, 1)),
            monthly_usd("Spotify", 9.99, d(2026, 4, 20)),
            monthly_usd("Netflix", 15.99, today()),
        ]);

        // All USD → uniform_currency → totals shown
        out += "\n=== totals with uniform currency ===\n";
        out += &run(&[
            monthly_usd("Netflix", 15.99, d(2026, 5, 1)),
            monthly_usd("Spotify", 9.99, d(2026, 5, 15)),
        ]);

        // No currency → uniform_currency returns None → no totals line
        out += "\n=== no currency, no totals ===\n";
        out += &run(&[Expense {
            name: "Rent".to_string(),
            amount: Some(1000.0),
            start_date: Some(d(2026, 5, 1)),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }]);

        // No amount or date → dashes in amount and due columns
        out += "\n=== incomplete expense ===\n";
        out += &run(&[Expense {
            name: "Unknown".to_string(),
            ..Default::default()
        }]);

        // None has end_date → no "ends" column rendered
        out += "\n=== no ends column when none set ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 5, 1))]);

        // Some have end_date → "ends" column appears, blank cells for those without
        out += "\n=== ends column with future end ===\n";
        out += &run(&[
            Expense {
                end_date: Some(d(2026, 6, 14)), // ~2 months
                ..monthly_usd("Trial", 9.99, d(2026, 5, 1))
            },
            monthly_usd("Netflix", 15.99, d(2026, 5, 1)),
        ]);

        // Past end_date → hidden by default, footer hint shown
        let with_ended = [
            Expense {
                end_date: Some(d(2026, 4, 5)), // 10 days ago
                ..monthly_usd("OldGym", 30.00, d(2025, 1, 1))
            },
            monthly_usd("Netflix", 15.99, d(2026, 4, 20)),
            Expense {
                end_date: Some(d(2026, 7, 15)), // ~3 months future
                ..monthly_usd("AnnualPlan", 50.00, d(2026, 5, 1))
            },
        ];

        out += "\n=== ended hidden by default ===\n";
        out += &run(&with_ended);

        // --all → ended rows visible, sink to bottom
        out += "\n=== --all reveals ended rows ===\n";
        out += &run_with(&with_ended, true);

        // All ended + default → friendly empty message
        out += "\n=== all ended + default hides everything ===\n";
        out += &run(&[Expense {
            end_date: Some(d(2026, 4, 5)),
            ..monthly_usd("OldGym", 30.00, d(2025, 1, 1))
        }]);

        insta::assert_snapshot!(out);
    }
}
