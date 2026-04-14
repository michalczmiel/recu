use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::expense::Expense;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct StoredExpense {
    name: String,
    amount: Option<f64>,
    currency: Option<String>,
    next_due: Option<chrono::NaiveDate>,
    interval: Option<crate::expense::Interval>,
    #[serde(default)]
    category: Option<String>,
}

impl StoredExpense {
    fn into_parts(self) -> (String, Expense) {
        (
            self.name,
            Expense {
                amount: self.amount,
                currency: self.currency,
                next_due: self.next_due,
                interval: self.interval,
                category: self.category,
            },
        )
    }
}

fn storage_file_from(value: Option<OsString>) -> PathBuf {
    value.map_or_else(|| PathBuf::from("recu.csv"), PathBuf::from)
}

fn storage_file() -> PathBuf {
    storage_file_from(std::env::var_os("RECU_FILE"))
}

fn io_invalid_data<E: std::error::Error + Send + Sync + 'static>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

fn read_all(path: &std::path::Path) -> io::Result<Vec<StoredExpense>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut reader = csv::Reader::from_path(path).map_err(io_invalid_data)?;
    reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_invalid_data)
}

fn write_all(path: &std::path::Path, entries: &[StoredExpense]) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("csv.tmp");
    {
        let mut writer = csv::Writer::from_path(&tmp_path).map_err(io_invalid_data)?;
        for entry in entries {
            writer.serialize(entry).map_err(io_invalid_data)?;
        }
        writer.flush()?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn list_entries(path: &std::path::Path) -> io::Result<Vec<StoredExpense>> {
    read_all(path)
}

pub fn save(name: &str, expense: &Expense) -> io::Result<PathBuf> {
    save_to(&storage_file(), name, expense)
}

pub(crate) fn save_to(
    path: &std::path::Path,
    name: &str,
    expense: &Expense,
) -> io::Result<PathBuf> {
    let mut entries = list_entries(path)?;
    if entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(name))
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("expense '{name}' already exists"),
        ));
    }

    entries.push(StoredExpense {
        name: name.to_string(),
        amount: expense.amount,
        currency: expense.currency.clone(),
        next_due: expense.next_due,
        interval: expense.interval.clone(),
        category: expense.category.clone(),
    });
    write_all(path, &entries)?;
    Ok(path.to_path_buf())
}

pub fn list() -> io::Result<Vec<(String, Expense)>> {
    list_from(&storage_file())
}

pub(crate) fn list_from(path: &std::path::Path) -> io::Result<Vec<(String, Expense)>> {
    Ok(list_entries(path)?
        .into_iter()
        .map(StoredExpense::into_parts)
        .collect())
}

fn resolve_index(path: &std::path::Path, target: &str) -> io::Result<usize> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid id"))?;
        let entries = list_entries(path)?;
        if id == 0 || id > entries.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no expense at @{id}"),
            ));
        }
        return Ok(id - 1);
    }

    let entries = list_entries(path)?;
    if let Some(index) = entries
        .iter()
        .position(|entry| entry.name.eq_ignore_ascii_case(target))
    {
        return Ok(index);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("expense '{target}' not found"),
    ))
}

pub fn update(target: &str, new_name: Option<&str>, patch: &Expense) -> io::Result<()> {
    update_from(&storage_file(), target, new_name, patch)
}

pub(crate) fn update_from(
    path: &std::path::Path,
    target: &str,
    new_name: Option<&str>,
    changes: &Expense,
) -> io::Result<()> {
    let index = resolve_index(path, target)?;
    let mut entries = list_entries(path)?;

    if let Some(name) = new_name
        && entries.iter().enumerate().any(|(other_index, entry)| {
            other_index != index && entry.name.eq_ignore_ascii_case(name)
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("expense '{name}' already exists"),
        ));
    }

    let expense = &mut entries[index];
    expense.amount = changes.amount.or(expense.amount);
    expense.currency = changes.currency.clone().or(expense.currency.clone());
    expense.next_due = changes.next_due.or(expense.next_due);
    expense.interval = changes.interval.clone().or(expense.interval.clone());
    expense.category = changes.category.clone().or(expense.category.clone());
    if let Some(name) = new_name {
        expense.name = name.to_string();
    }

    write_all(path, &entries)?;

    Ok(())
}

pub fn remove(target: &str) -> io::Result<String> {
    remove_from(&storage_file(), target)
}

pub(crate) fn remove_from(path: &std::path::Path, target: &str) -> io::Result<String> {
    let index = resolve_index(path, target)?;
    let mut entries = list_entries(path)?;
    let name = entries.remove(index).name;
    write_all(path, &entries)?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_rejects_duplicate_case_insensitive() {
        let file = std::env::temp_dir().join("recu-test-storage-dup.csv");
        let _ = fs::remove_file(&file);

        let expense = Expense {
            amount: Some(9.99),
            currency: Some("usd".into()),
            ..Default::default()
        };

        save_to(&file, "Netflix", &expense).expect("first save should succeed");

        let err = save_to(&file, "netflix", &expense).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);

        assert_eq!(list_from(&file).expect("list should succeed").len(), 1);
    }

    #[test]
    fn list_from_missing_file_returns_empty() {
        let file = std::env::temp_dir().join("recu-test-storage-missing.csv");
        let _ = fs::remove_file(&file);

        assert!(list_from(&file).expect("list should succeed").is_empty());
    }

    #[test]
    fn storage_file_defaults_to_local_csv() {
        assert_eq!(storage_file_from(None), PathBuf::from("recu.csv"));
    }

    #[test]
    fn storage_file_uses_env_override() {
        let override_path = std::env::temp_dir().join("custom-recu.csv");
        assert_eq!(
            storage_file_from(Some(override_path.clone().into_os_string())),
            override_path
        );
    }
}
