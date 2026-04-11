use chrono::{Datelike, NaiveDate};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl Interval {
    pub fn next_payment(&self, first: NaiveDate, today: NaiveDate) -> NaiveDate {
        match self {
            Interval::Weekly => {
                let days_since = (today - first).num_days().rem_euclid(7);
                if days_since == 0 {
                    today
                } else {
                    today + chrono::Days::new((7 - days_since) as u64)
                }
            }
            Interval::Monthly => advance_months(first, today, 1),
            Interval::Quarterly => advance_months(first, today, 3),
            Interval::Yearly => advance_months(first, today, 12),
        }
    }
}

fn advance_months(first: NaiveDate, today: NaiveDate, step: u32) -> NaiveDate {
    let step = step as i32;
    let diff = (today.year() - first.year()) * 12
        + (today.month() as i32 - first.month() as i32);
    let mut k = (diff.max(0) / step) * step;
    loop {
        let candidate = month_offset(first, k);
        if candidate >= today {
            return candidate;
        }
        k += step;
    }
}

fn month_offset(first: NaiveDate, months: i32) -> NaiveDate {
    let total = first.year() * 12 + (first.month() as i32 - 1) + months;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    for day in (1..=first.day()).rev() {
        if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
            return d;
        }
    }
    first
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub tags: Option<Vec<String>>,
    pub first_payment_date: Option<NaiveDate>,
    pub interval: Option<Interval>,
}

impl Expense {
    pub fn next_payment(&self, today: NaiveDate) -> Option<NaiveDate> {
        let first = self.first_payment_date?;
        let interval = self.interval.as_ref()?;
        Some(interval.next_payment(first, today))
    }

    pub fn days_until_next(&self, today: NaiveDate) -> Option<i64> {
        Some((self.next_payment(today)? - today).num_days())
    }
}

fn storage_dir() -> io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(".cache").join("recu"))
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

pub fn save(name: &str, expense: &Expense) -> io::Result<PathBuf> {
    save_to(&storage_dir()?, name, expense)
}

pub(crate) fn save_to(
    dir: &std::path::Path,
    name: &str,
    expense: &Expense,
) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let slug = slugify(name);
    let path = dir.join(format!("{}.md", slug));

    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("expense '{}' already exists", name),
        ));
    }

    let frontmatter = serde_yaml::to_string(expense)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let content = format!("---\n{}---\n# {}\n", frontmatter, name);

    fs::write(&path, content)?;
    Ok(path)
}

pub fn list() -> io::Result<Vec<(String, Expense)>> {
    list_from(&storage_dir()?)
}

pub(crate) fn list_from(dir: &std::path::Path) -> io::Result<Vec<(String, Expense)>> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut expenses = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            if let Some(parsed) = parse_file(&path) {
                expenses.push(parsed);
            }
        }
    }
    expenses.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(expenses)
}

pub fn remove(target: &str) -> io::Result<()> {
    remove_from(&storage_dir()?, target)
}

pub(crate) fn remove_from(dir: &std::path::Path, target: &str) -> io::Result<()> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid id"))?;
        let entries = list_from(dir)?;
        if id == 0 || id > entries.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no expense at @{}", id),
            ));
        }
        let name = &entries[id - 1].0;
        let slug = slugify(name);
        return fs::remove_file(dir.join(format!("{}.md", slug)));
    }

    let path = dir.join(format!("{}.md", slugify(target)));
    if path.exists() {
        return fs::remove_file(path);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("expense '{}' not found", target),
    ))
}

fn parse_file(path: &std::path::Path) -> Option<(String, Expense)> {
    let content = fs::read_to_string(path).ok()?;
    let content = content.strip_prefix("---\n")?;
    let end = content.find("---\n")?;
    let yaml = &content[..end];
    let rest = &content[end + 4..];

    let expense: Expense = serde_yaml::from_str(yaml).ok()?;
    let name = rest
        .lines()
        .find(|l| l.starts_with("# "))?
        .strip_prefix("# ")?
        .to_string();

    Some((name, expense))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_simple() {
        assert_eq!(slugify("Netflix"), "netflix");
    }

    #[test]
    fn slugify_multi_word() {
        assert_eq!(slugify("NY Times"), "ny-times");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("Gym & Spa!"), "gym--spa");
    }

    #[test]
    fn save_rejects_duplicate_slug() {
        let dir = std::env::temp_dir().join("recu-test-storage-dup");
        let _ = fs::remove_dir_all(&dir);

        let expense = Expense {
            amount: Some(9.99),
            currency: Some("usd".into()),
            tags: None,
            first_payment_date: None,
            interval: None,
        };

        save_to(&dir, "Netflix", &expense).unwrap();

        let err = save_to(&dir, "netflix", &expense).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);

        assert_eq!(list_from(&dir).unwrap().len(), 1);
    }
}
