use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    use chrono::Datelike;
    let mut year = first.year();
    let mut month = first.month();
    let day = first.day();

    loop {
        let candidate = NaiveDate::from_ymd_opt(year, month, day)
            .or_else(|| {
                // handle day overflow (e.g. Jan 31 -> Feb 28)
                let last = last_day_of_month(year, month);
                NaiveDate::from_ymd_opt(year, month, last)
            })
            .unwrap();
        if candidate >= today {
            return candidate;
        }
        month += step;
        while month > 12 {
            month -= 12;
            year += 1;
        }
    }
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    use chrono::Datelike;
    if month == 12 {
        31
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap()
            .pred_opt()
            .unwrap()
            .day()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub tags: Option<Vec<String>>,
    pub first_payment_date: Option<String>,
    pub interval: Option<Interval>,
}

fn storage_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".cache").join("recu")
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

pub fn save(name: &str, expense: &Expense) -> std::io::Result<PathBuf> {
    save_to(&storage_dir(), name, expense)
}

pub(crate) fn save_to(
    dir: &std::path::Path,
    name: &str,
    expense: &Expense,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;

    let slug = slugify(name);
    let path = dir.join(format!("{}.md", slug));

    let frontmatter = serde_yaml::to_string(expense).expect("failed to serialize expense");
    let content = format!("---\n{}---\n# {}\n", frontmatter, name);

    fs::write(&path, content)?;
    Ok(path)
}

pub fn list() -> std::io::Result<Vec<(String, Expense)>> {
    list_from(&storage_dir())
}

pub(crate) fn list_from(dir: &std::path::Path) -> std::io::Result<Vec<(String, Expense)>> {
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
    Ok(expenses)
}

pub fn remove(target: &str) -> std::io::Result<()> {
    remove_from(&storage_dir(), target)
}

pub(crate) fn remove_from(dir: &std::path::Path, target: &str) -> std::io::Result<()> {
    // Handle @id syntax
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid id"))?;
        let entries = list_from(dir)?;
        if id == 0 || id > entries.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
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

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
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
}
