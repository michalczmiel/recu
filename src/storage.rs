use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub tags: Option<Vec<String>>,
    pub first_payment_date: Option<String>,
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
