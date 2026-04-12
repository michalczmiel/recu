use std::fs;
use std::io;
use std::path::PathBuf;

use crate::expense::Expense;

fn storage_dir() -> io::Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("recu"));
    }
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(".local").join("share").join("recu"))
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

pub(crate) fn save_to(dir: &std::path::Path, name: &str, expense: &Expense) -> io::Result<PathBuf> {
    let slug = slugify(name);
    let path = dir.join(format!("{}.md", slug));

    fs::create_dir_all(dir)?;

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

fn collect_entries(
    dir: &std::path::Path,
    entries: &mut Vec<(String, Expense, PathBuf)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_entries(&path, entries)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            if let Some((name, expense)) = parse_file(&path) {
                entries.push((name, expense, path));
            }
        }
    }
    Ok(())
}

fn list_entries(dir: &std::path::Path) -> io::Result<Vec<(String, Expense, PathBuf)>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut entries = Vec::new();
    collect_entries(dir, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

pub fn list() -> io::Result<Vec<(String, Expense)>> {
    list_from(&storage_dir()?)
}

pub(crate) fn list_from(dir: &std::path::Path) -> io::Result<Vec<(String, Expense)>> {
    Ok(list_entries(dir)?
        .into_iter()
        .map(|(name, expense, _)| (name, expense))
        .collect())
}

fn resolve_path(dir: &std::path::Path, target: &str) -> io::Result<PathBuf> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid id"))?;
        let entries = list_entries(dir)?;
        if id == 0 || id > entries.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no expense at @{}", id),
            ));
        }
        return Ok(entries[id - 1].2.clone());
    }

    let path = dir.join(format!("{}.md", slugify(target)));
    if path.exists() {
        return Ok(path);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("expense '{}' not found", target),
    ))
}

pub fn update(target: &str, new_name: Option<&str>, patch: &Expense) -> io::Result<()> {
    update_from(&storage_dir()?, target, new_name, patch)
}

pub(crate) fn update_from(
    dir: &std::path::Path,
    target: &str,
    new_name: Option<&str>,
    patch: &Expense,
) -> io::Result<()> {
    let path = resolve_path(dir, target)?;

    // check rename target doesn't conflict before touching anything
    let new_path = if let Some(name) = new_name {
        let p = dir.join(format!("{}.md", slugify(name)));
        if p != path && p.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("expense '{}' already exists", name),
            ));
        }
        Some(p)
    } else {
        None
    };

    let content = fs::read_to_string(&path)?;
    let (yaml, rest) = parse_frontmatter(&content)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid file format"))?;

    let mut expense: Expense =
        serde_yaml::from_str(yaml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    expense.amount = patch.amount.or(expense.amount);
    expense.currency = patch.currency.clone().or(expense.currency);
    expense.tags = patch.tags.clone().or(expense.tags);
    expense.first_payment_date = patch.first_payment_date.or(expense.first_payment_date);
    expense.interval = patch.interval.clone().or(expense.interval);

    let display_name = new_name.unwrap_or_else(|| {
        rest.lines()
            .find(|l| l.starts_with("# "))
            .and_then(|l| l.strip_prefix("# "))
            .unwrap_or("")
    });

    let frontmatter = serde_yaml::to_string(&expense)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let new_content = format!("---\n{}---\n# {}\n", frontmatter, display_name);

    match new_path {
        Some(ref p) if p != &path => {
            fs::write(p, new_content)?;
            fs::remove_file(&path)?;
        }
        _ => fs::write(&path, new_content)?,
    }

    Ok(())
}

pub fn remove(target: &str) -> io::Result<String> {
    remove_from(&storage_dir()?, target)
}

pub(crate) fn remove_from(dir: &std::path::Path, target: &str) -> io::Result<String> {
    let path = resolve_path(dir, target)?;
    let (name, _) = parse_file(&path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "could not read expense"))?;
    fs::remove_file(&path)?;
    Ok(name)
}

fn parse_frontmatter(content: &str) -> Option<(&str, &str)> {
    let content = content.strip_prefix("---\n")?;
    let end = content.find("---\n")?;
    Some((&content[..end], &content[end + 4..]))
}

fn parse_file(path: &std::path::Path) -> Option<(String, Expense)> {
    let content = fs::read_to_string(path).ok()?;
    let (yaml, rest) = parse_frontmatter(&content)?;
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
