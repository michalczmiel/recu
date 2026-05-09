use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use crate::config::{self, Config};
use crate::expense::{self, Expense, convert, find_currency, format_amount, round_money};
use crate::rates;
use crate::store::Store;
use crate::ui;
use chrono::{Datelike, Months, NaiveDate, Weekday};
use clap::Args;
use colored::Colorize;
use rusty_money::iso;
use serde::Serialize;

use crate::commands::OutputFormat;

const CELL_WIDTH: usize = 7;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu calendar
  recu calendar --next
  recu calendar --month 2026-12")]
pub struct CalendarArgs {
    /// Show next month
    #[arg(long)]
    pub next: bool,
    /// Show a specific month (YYYY-MM)
    #[arg(long, value_name = "YYYY-MM", value_parser = parse_month)]
    pub month: Option<NaiveDate>,
    /// Include ended expenses
    #[arg(short, long)]
    pub all: bool,
    /// Filter by category (case-insensitive); comma-separated for multiple
    #[arg(short, long, value_delimiter = ',')]
    pub category: Vec<String>,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

fn parse_month(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d")
        .map_err(|_| format!("'{s}' is not a valid YYYY-MM month"))
}

#[derive(Default, Clone)]
struct DayCell {
    total: f64,
    count: u32,
}

#[derive(Clone)]
struct Charge {
    id: u64,
    name: String,
    amount: f64,
}

fn first_of_month(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("day 1 always valid")
}

fn last_of_month(d: NaiveDate) -> NaiveDate {
    first_of_month(d)
        .checked_add_months(Months::new(1))
        .and_then(|d| d.pred_opt())
        .expect("month + 1 in chrono range")
}

fn month_label(d: NaiveDate) -> String {
    d.format("%B %Y").to_string()
}

fn charges_for_month(
    expenses: &[Expense],
    month: NaiveDate,
    today: NaiveDate,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    include_ended: bool,
) -> BTreeMap<NaiveDate, Vec<Charge>> {
    let start = first_of_month(month);
    let end = last_of_month(month);
    let mut by_day: BTreeMap<NaiveDate, Vec<Charge>> = BTreeMap::new();

    for exp in expenses {
        if !include_ended && exp.is_ended(today) {
            continue;
        }
        let (Some(first), Some(amount)) = (exp.start_date, exp.amount) else {
            continue;
        };
        let converted = convert(amount, exp.currency.as_deref(), rates, target);
        let charge = || Charge {
            id: exp.id,
            name: exp.name.clone(),
            amount: converted,
        };

        if let Some(interval) = exp.interval.as_ref() {
            let mut d = interval.next_payment(first, start);
            while d <= end {
                by_day.entry(d).or_default().push(charge());
                d = interval.next_payment(first, d + chrono::Days::new(1));
            }
        } else if first >= start && first <= end {
            by_day.entry(first).or_default().push(charge());
        }
    }

    by_day
}

fn cells_from_charges(by_day: &BTreeMap<NaiveDate, Vec<Charge>>) -> BTreeMap<NaiveDate, DayCell> {
    by_day
        .iter()
        .map(|(d, cs)| {
            (
                *d,
                DayCell {
                    total: cs.iter().map(|c| c.amount).sum(),
                    count: u32::try_from(cs.len()).unwrap_or(u32::MAX),
                },
            )
        })
        .collect()
}

fn format_int_with_spaces(n: f64) -> String {
    let sign = if n < 0.0 { "-" } else { "" };
    let abs = format!("{:.0}", n.abs());
    let groups: Vec<&str> = abs
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).expect("ascii digits"))
        .collect();
    format!("{sign}{}", groups.join(" "))
}

fn format_amount_cell(total: f64, room: usize) -> String {
    let abs = total.abs();
    let fits = |s: &str| s.chars().count() <= room;

    let full = format_int_with_spaces(total);
    if fits(&full) {
        return full;
    }
    if abs >= 10_000.0 {
        let k = format!("{:.0}k", abs / 1000.0);
        if fits(&k) {
            return k;
        }
    }
    if abs >= 1_000.0 {
        let k = format!("{:.1}k", abs / 1000.0);
        if fits(&k) {
            return k;
        }
    }
    full.chars().take(room).collect()
}

#[derive(Clone, Copy)]
struct DayState {
    is_today: bool,
    is_past: bool,
}

fn day_cell(day: u32, state: DayState) -> String {
    let base = format!("{day:>CELL_WIDTH$}");
    if state.is_today {
        base.yellow().bold().to_string()
    } else if state.is_past {
        base.dimmed().to_string()
    } else {
        base
    }
}

fn amount_cell(cell: Option<&DayCell>, state: DayState) -> String {
    let Some(c) = cell else {
        return " ".repeat(CELL_WIDTH);
    };
    let badge = if c.count > 1 {
        format!("({})", c.count)
    } else {
        String::new()
    };
    let room = CELL_WIDTH.saturating_sub(badge.len());
    let amt = format_amount_cell(c.total, room);
    let combined = format!("{amt}{badge}");
    let padded = format!("{combined:>CELL_WIDTH$}");
    if state.is_past {
        padded.dimmed().bold().to_string()
    } else {
        padded.cyan().bold().to_string()
    }
}

fn weeks_for_month(month: NaiveDate) -> Vec<Vec<Option<NaiveDate>>> {
    let first = first_of_month(month);
    let last = last_of_month(month);
    let mut weeks: Vec<Vec<Option<NaiveDate>>> = Vec::new();
    let mut current_week: Vec<Option<NaiveDate>> = Vec::new();

    let lead = first.weekday().num_days_from_monday();
    for _ in 0..lead {
        current_week.push(None);
    }

    let mut d = first;
    while d <= last {
        current_week.push(Some(d));
        if d.weekday() == Weekday::Sun {
            weeks.push(std::mem::take(&mut current_week));
        }
        d = d.succ_opt().expect("date in chrono range");
    }
    if !current_week.is_empty() {
        weeks.push(current_week);
    }
    weeks
}

fn render_grid(
    out: &mut impl Write,
    month: NaiveDate,
    today: NaiveDate,
    by_day: &BTreeMap<NaiveDate, DayCell>,
) -> std::io::Result<()> {
    let title = month_label(month);
    let total_w = CELL_WIDTH * 7;
    let pad_left = total_w.saturating_sub(title.chars().count()) / 2;
    writeln!(out, "{}{}", " ".repeat(pad_left), ui::heading(&title))?;
    writeln!(out)?;

    for name in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
        write!(out, "{name:>CELL_WIDTH$}")?;
    }
    writeln!(out)?;

    let is_current_month = today.year() == month.year() && today.month() == month.month();

    for week in weeks_for_month(month) {
        let mut date_line = String::new();
        let mut amount_line = String::new();
        let mut any_charge = false;

        for slot in &week {
            match slot {
                None => {
                    date_line.push_str(&" ".repeat(CELL_WIDTH));
                    amount_line.push_str(&" ".repeat(CELL_WIDTH));
                }
                Some(d) => {
                    let cell = by_day.get(d);
                    if cell.is_some() {
                        any_charge = true;
                    }
                    let state = DayState {
                        is_today: is_current_month && *d == today,
                        is_past: is_current_month && *d < today,
                    };
                    date_line.push_str(&day_cell(d.day(), state));
                    amount_line.push_str(&amount_cell(cell, state));
                }
            }
        }
        writeln!(out, "{date_line}")?;
        if any_charge {
            writeln!(out, "{amount_line}")?;
        }
    }
    Ok(())
}

fn print_footer(
    out: &mut impl Write,
    by_day: &BTreeMap<NaiveDate, DayCell>,
    target_cur: Option<&'static iso::Currency>,
    month: NaiveDate,
    today: NaiveDate,
    hidden_ended: usize,
) -> std::io::Result<()> {
    let count: u32 = by_day.values().map(|c| c.count).sum();
    let total: f64 = by_day.values().map(|c| c.total).sum();

    if let Some(cur) = target_cur
        && count > 0
    {
        let charges_label = if count == 1 { "charge" } else { "charges" };
        let is_current_month = today.year() == month.year() && today.month() == month.month();
        let line = if is_current_month {
            let (paid, remaining): (f64, f64) = by_day.iter().fold((0.0, 0.0), |(p, r), (d, c)| {
                if *d < today {
                    (p + c.total, r)
                } else {
                    (p, r + c.total)
                }
            });
            format!(
                "\n{count} {charges_label}   {}   paid {}, remaining {}",
                format_amount(cur, total),
                format_amount(cur, paid),
                format_amount(cur, remaining),
            )
        } else {
            format!("\n{count} {charges_label}   {}", format_amount(cur, total))
        };
        writeln!(out, "{}", ui::heading(&line))?;
    }

    if hidden_ended > 0 {
        writeln!(
            out,
            "{}",
            ui::dim(&format!("+ {hidden_ended} ended (recu calendar --all)"))
        )?;
    }
    Ok(())
}

#[derive(Serialize)]
struct JsonCharge<'a> {
    id: u64,
    name: &'a str,
    amount: f64,
}

#[derive(Serialize)]
struct JsonDay<'a> {
    date: NaiveDate,
    total: f64,
    charges: Vec<JsonCharge<'a>>,
}

#[derive(Serialize)]
struct JsonCalendar<'a> {
    month: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<&'a str>,
    total: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    paid: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining: Option<f64>,
    days: Vec<JsonDay<'a>>,
}

fn execute_json(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[Expense],
    month: NaiveDate,
    all: bool,
    categories: &[String],
) -> std::io::Result<()> {
    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(find_currency)
        .or_else(|| expense::uniform_currency(expenses));

    let filtered: Vec<Expense> = expenses
        .iter()
        .filter(|e| expense::matches_categories(e, categories))
        .cloned()
        .collect();

    let by_day = charges_for_month(
        &filtered,
        month,
        today,
        exchange_rates.as_ref(),
        target,
        all,
    );

    let total: f64 = by_day
        .values()
        .flat_map(|cs| cs.iter().map(|c| c.amount))
        .sum();

    let is_current = today.year() == month.year() && today.month() == month.month();
    let (paid, remaining) = if is_current {
        let (p, r) = by_day.iter().fold((0.0, 0.0), |(p, r), (d, cs)| {
            let day_total: f64 = cs.iter().map(|c| c.amount).sum();
            if *d < today {
                (p + day_total, r)
            } else {
                (p, r + day_total)
            }
        });
        (Some(round_money(p)), Some(round_money(r)))
    } else {
        (None, None)
    };

    let days: Vec<JsonDay<'_>> = by_day
        .iter()
        .map(|(d, cs)| JsonDay {
            date: *d,
            total: round_money(cs.iter().map(|c| c.amount).sum()),
            charges: cs
                .iter()
                .map(|c| JsonCharge {
                    id: c.id,
                    name: &c.name,
                    amount: round_money(c.amount),
                })
                .collect(),
        })
        .collect();

    let envelope = JsonCalendar {
        month: month.format("%Y-%m").to_string(),
        currency: target_cur.map(|c| c.iso_alpha_code),
        total: round_money(total),
        paid,
        remaining,
        days,
    };

    serde_json::to_writer_pretty(&mut *out, &envelope)?;
    writeln!(out)
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[Expense],
    month: NaiveDate,
    all: bool,
    categories: &[String],
) -> std::io::Result<()> {
    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(find_currency)
        .or_else(|| expense::uniform_currency(expenses));

    let filtered: Vec<Expense> = expenses
        .iter()
        .filter(|e| expense::matches_categories(e, categories))
        .cloned()
        .collect();

    let by_day_charges = charges_for_month(
        &filtered,
        month,
        today,
        exchange_rates.as_ref(),
        target,
        all,
    );
    let by_day = cells_from_charges(&by_day_charges);

    let hidden_ended = if all {
        0
    } else {
        filtered.iter().filter(|e| e.is_ended(today)).count()
    };

    render_grid(out, month, today, &by_day)?;
    print_footer(out, &by_day, target_cur, month, today, hidden_ended)?;
    Ok(())
}

pub fn execute(args: &CalendarArgs, store: &Store) -> std::io::Result<()> {
    let expenses = store.list()?;
    let cfg = config::load()?;
    let today = chrono::Local::now().date_naive();

    let month = if let Some(m) = args.month {
        m
    } else if args.next {
        first_of_month(today)
            .checked_add_months(Months::new(1))
            .expect("month + 1 in chrono range")
    } else {
        first_of_month(today)
    };

    let categories = crate::commands::category::resolve_filter(&args.category, store)?;
    let mut out = std::io::stdout();
    match args.format {
        OutputFormat::Json => execute_json(
            &mut out,
            today,
            &cfg,
            &expenses,
            month,
            args.all,
            &categories,
        ),
        OutputFormat::Text => execute_with(
            &mut out,
            today,
            &cfg,
            &expenses,
            month,
            args.all,
            &categories,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn today() -> NaiveDate {
        d(2026, 11, 14)
    }

    fn monthly_pln(name: &str, amount: f64, start: NaiveDate) -> Expense {
        Expense {
            name: name.to_string(),
            amount: Some(amount),
            currency: Some("pln".to_string()),
            start_date: Some(start),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }
    }

    fn run(month: NaiveDate, expenses: &[Expense]) -> String {
        let mut buf = Vec::new();
        let cfg = Config {
            currency: Some("pln".to_string()),
        };
        execute_with(&mut buf, today(), &cfg, expenses, month, false, &[]).expect("execute_with");
        String::from_utf8(buf).expect("utf8")
    }

    #[test]
    fn calendar() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let expenses = vec![
            monthly_pln("Internet", 50.0, d(2026, 11, 6)),
            monthly_pln("Rent", 4766.0, d(2026, 11, 13)),
            monthly_pln("Utilities", 4766.0, d(2026, 11, 13)),
            monthly_pln("Phone", 230.0, d(2026, 11, 20)),
            monthly_pln("Streaming1", 1206.0, d(2026, 11, 27)),
            monthly_pln("Streaming2", 1206.0, d(2026, 11, 27)),
            monthly_pln("Gym", 89.0, d(2026, 11, 28)),
            monthly_pln("Books", 89.0, d(2026, 11, 28)),
        ];

        let mut out = String::new();
        out += "=== current month (no color, utf8) ===\n";
        out += &run(d(2026, 11, 1), &expenses);

        out += "\n=== next month ===\n";
        out += &run(d(2026, 12, 1), &expenses);

        out += "\n=== empty month ===\n";
        out += &run(d(2026, 11, 1), &[]);

        insta::assert_snapshot!(out);
    }

    #[test]
    fn parses_month_arg() {
        assert_eq!(parse_month("2026-12"), Ok(d(2026, 12, 1)));
        assert!(parse_month("not-a-month").is_err());
    }

    #[test]
    fn format_int_thousands_space() {
        assert_eq!(format_int_with_spaces(50.0), "50");
        assert_eq!(format_int_with_spaces(4766.0), "4 766");
        assert_eq!(format_int_with_spaces(1_234_567.0), "1 234 567");
    }

    #[test]
    fn amount_cell_abbreviates_when_too_wide() {
        assert_eq!(format_amount_cell(4766.0, 7), "4 766");
        assert_eq!(format_amount_cell(4766.0, 5), "4 766");
        assert_eq!(format_amount_cell(12_345.0, 5), "12k");
        assert_eq!(format_amount_cell(1234.0, 4), "1.2k");
    }
}
