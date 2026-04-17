use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::expense::Expense;

fn storage_file_from(value: Option<OsString>) -> PathBuf {
    value.map_or_else(|| PathBuf::from("recu.csv"), PathBuf::from)
}

fn storage_file() -> PathBuf {
    storage_file_from(std::env::var_os("RECU_FILE"))
}

fn io_invalid_data<E: std::error::Error + Send + Sync + 'static>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

fn read_all(path: &std::path::Path) -> io::Result<Vec<Expense>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut reader = csv::Reader::from_path(path).map_err(io_invalid_data)?;
    reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_invalid_data)
}

fn undo_path(path: &std::path::Path) -> PathBuf {
    path.with_extension("csv.undo")
}

fn snapshot(path: &std::path::Path) -> io::Result<()> {
    if path.exists() {
        fs::copy(path, undo_path(path))?;
    }
    Ok(())
}

fn diff_description(before: &[Expense], after: &[Expense]) -> String {
    match after.len().cmp(&before.len()) {
        std::cmp::Ordering::Greater => {
            if let Some(e) = after
                .iter()
                .find(|a| !before.iter().any(|b| b.name.eq_ignore_ascii_case(&a.name)))
            {
                return format!("Undid add of '{}'", e.name);
            }
        }
        std::cmp::Ordering::Less => {
            if let Some(e) = before
                .iter()
                .find(|b| !after.iter().any(|a| a.name.eq_ignore_ascii_case(&b.name)))
            {
                return format!("Restored '{}'", e.name);
            }
        }
        std::cmp::Ordering::Equal => {
            for b in before {
                let changed =
                    after.iter().find(|a| a.name.eq_ignore_ascii_case(&b.name)) != Some(b);
                if changed {
                    return format!("Reverted edit of '{}'", b.name);
                }
            }
        }
    }
    "Undone".to_string()
}

pub fn restore() -> io::Result<String> {
    restore_from(&storage_file())
}

pub(crate) fn restore_from(path: &std::path::Path) -> io::Result<String> {
    let undo = undo_path(path);
    if !undo.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "nothing to undo"));
    }
    let before = read_all(&undo)?;
    let after = read_all(path)?;
    let msg = diff_description(&before, &after);
    fs::rename(&undo, path)?;
    Ok(msg)
}

fn write_all(path: &std::path::Path, entries: &[Expense]) -> io::Result<()> {
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

pub fn save(expense: &Expense) -> io::Result<PathBuf> {
    save_to(&storage_file(), expense)
}

pub(crate) fn save_to(path: &std::path::Path, expense: &Expense) -> io::Result<PathBuf> {
    snapshot(path)?;
    let mut entries = read_all(path)?;
    if entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(&expense.name))
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("expense '{}' already exists", expense.name),
        ));
    }

    entries.push(expense.clone());
    write_all(path, &entries)?;
    Ok(path.to_path_buf())
}

pub fn list() -> io::Result<Vec<Expense>> {
    list_from(&storage_file())
}

pub(crate) fn list_from(path: &std::path::Path) -> io::Result<Vec<Expense>> {
    read_all(path)
}

pub fn categories() -> io::Result<Vec<String>> {
    categories_from(&storage_file())
}

pub(crate) fn categories_from(path: &std::path::Path) -> io::Result<Vec<String>> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut categories = Vec::<String>::new();
    for expense in list_from(path)? {
        let Some(category) = expense.category else {
            continue;
        };
        if seen.insert(category.to_ascii_lowercase()) {
            categories.push(category);
        }
    }
    categories.sort_by_cached_key(|category| category.to_ascii_lowercase());
    Ok(categories)
}

fn resolve_index_in(entries: &[Expense], target: &str) -> io::Result<usize> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid id"))?;
        if id == 0 || id > entries.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no expense at @{id}"),
            ));
        }
        return Ok(id - 1);
    }

    entries
        .iter()
        .position(|entry| entry.name.eq_ignore_ascii_case(target))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("expense '{target}' not found"),
            )
        })
}

fn resolve_index(path: &std::path::Path, target: &str) -> io::Result<usize> {
    resolve_index_in(&read_all(path)?, target)
}

pub fn get(target: &str) -> io::Result<Expense> {
    get_from(&storage_file(), target)
}

pub(crate) fn get_from(path: &std::path::Path, target: &str) -> io::Result<Expense> {
    let index = resolve_index(path, target)?;
    read_all(path)?
        .into_iter()
        .nth(index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
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
    snapshot(path)?;
    let index = resolve_index(path, target)?;
    let mut entries = read_all(path)?;

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
    expense.currency = changes
        .currency
        .as_ref()
        .or(expense.currency.as_ref())
        .cloned();
    expense.start_date = changes.start_date.or(expense.start_date);
    expense.interval = changes
        .interval
        .as_ref()
        .or(expense.interval.as_ref())
        .cloned();
    expense.category = changes
        .category
        .as_ref()
        .or(expense.category.as_ref())
        .cloned();
    if let Some(name) = new_name {
        expense.name = name.to_string();
    }

    write_all(path, &entries)?;

    Ok(())
}

pub fn remove(targets: &[&str]) -> io::Result<Vec<String>> {
    remove_from(&storage_file(), targets)
}

pub(crate) fn remove_from(path: &std::path::Path, targets: &[&str]) -> io::Result<Vec<String>> {
    snapshot(path)?;
    let mut entries = read_all(path)?;

    let mut indices: Vec<usize> = Vec::with_capacity(targets.len());
    for target in targets {
        let index = resolve_index_in(&entries, target)?;
        if indices.contains(&index) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("duplicate target: {target}"),
            ));
        }
        indices.push(index);
    }

    // Remove highest indices first so earlier removals don't shift remaining ones
    let mut order: Vec<usize> = (0..indices.len()).collect();
    order.sort_unstable_by(|&a, &b| indices[b].cmp(&indices[a]));

    let mut names = vec![String::new(); indices.len()];
    for pos in order {
        names[pos] = entries.remove(indices[pos]).name;
    }

    write_all(path, &entries)?;
    Ok(names)
}

pub fn clear_category(category: &str) -> io::Result<usize> {
    clear_category_from(&storage_file(), category)
}

pub(crate) fn clear_category_from(path: &std::path::Path, category: &str) -> io::Result<usize> {
    let mut entries = read_all(path)?;
    let updated = entries
        .iter_mut()
        .filter(|entry| {
            entry
                .category
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case(category))
        })
        .map(|entry| {
            entry.category = None;
        })
        .count();

    if updated == 0 {
        return Ok(0);
    }

    snapshot(path)?;
    write_all(path, &entries)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str, amount: f64) -> Expense {
        Expense {
            name: name.to_string(),
            amount: Some(amount),
            currency: Some("usd".into()),
            ..Default::default()
        }
    }

    #[test]
    fn save_rejects_duplicate_case_insensitive() -> io::Result<()> {
        let file = std::env::temp_dir().join("recu-test-storage-dup.csv");
        let _ = fs::remove_file(&file);

        save_to(&file, &named("Netflix", 9.99))?;

        let err = save_to(&file, &named("netflix", 9.99)).expect_err("duplicate save should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);

        assert_eq!(list_from(&file)?.len(), 1);
        Ok(())
    }

    #[test]
    fn list_from_missing_file_returns_empty() -> io::Result<()> {
        let file = std::env::temp_dir().join("recu-test-storage-missing.csv");
        let _ = fs::remove_file(&file);

        assert!(list_from(&file)?.is_empty());
        Ok(())
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

    fn make_test_file(name: &str) -> PathBuf {
        let file = std::env::temp_dir().join(format!("recu-test-undo-{name}.csv"));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(undo_path(&file));
        file
    }

    #[test]
    fn restore_after_remove() -> io::Result<()> {
        let file = make_test_file("remove");
        save_to(&file, &named("Netflix", 9.99))?;
        remove_from(&file, &["Netflix"])?;
        assert!(list_from(&file)?.is_empty());
        let msg = restore_from(&file)?;
        assert_eq!(msg, "Restored 'Netflix'");
        assert_eq!(list_from(&file)?.len(), 1);
        Ok(())
    }

    #[test]
    fn restore_after_update() -> io::Result<()> {
        let file = make_test_file("update");
        save_to(&file, &named("Netflix", 9.99))?;
        update_from(&file, "Netflix", None, &named("Netflix", 14.99))?;
        assert_eq!(list_from(&file)?[0].amount, Some(14.99));
        let msg = restore_from(&file)?;
        assert_eq!(msg, "Reverted edit of 'Netflix'");
        assert_eq!(list_from(&file)?[0].amount, Some(9.99));
        Ok(())
    }

    #[test]
    fn restore_after_add() -> io::Result<()> {
        let file = make_test_file("add");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        assert_eq!(list_from(&file)?.len(), 2);
        let msg = restore_from(&file)?;
        assert_eq!(msg, "Undid add of 'Spotify'");
        assert_eq!(list_from(&file)?.len(), 1);
        Ok(())
    }

    #[test]
    fn restore_with_no_snapshot_returns_error() -> io::Result<()> {
        let file = make_test_file("nosnap");
        save_to(&file, &named("Netflix", 9.99))?;
        let err = restore_from(&file).expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn restore_is_single_use() -> io::Result<()> {
        let file = make_test_file("singleuse");
        save_to(&file, &named("Netflix", 9.99))?;
        remove_from(&file, &["Netflix"])?;
        restore_from(&file)?;
        let err = restore_from(&file).expect_err("second restore should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn categories_from_lists_unique_categories_case_insensitive() -> io::Result<()> {
        let file = make_test_file("categories");
        save_to(
            &file,
            &Expense {
                category: Some("Streaming".into()),
                ..named("Netflix", 9.99)
            },
        )?;
        save_to(
            &file,
            &Expense {
                category: Some("streaming".into()),
                ..named("Spotify", 5.99)
            },
        )?;
        save_to(
            &file,
            &Expense {
                category: Some("Housing".into()),
                ..named("Rent", 999.0)
            },
        )?;

        assert_eq!(categories_from(&file)?, vec!["Housing", "Streaming"]);
        Ok(())
    }

    #[test]
    fn remove_by_id() -> io::Result<()> {
        let file = make_test_file("remove-by-id");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        let names = remove_from(&file, &["@1"])?;
        assert_eq!(names, vec!["Netflix"]);
        let entries = list_from(&file)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn update_by_id() -> io::Result<()> {
        let file = make_test_file("update-by-id");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        update_from(&file, "@2", None, &named("Spotify", 7.99))?;
        assert_eq!(list_from(&file)?[1].amount, Some(7.99));
        Ok(())
    }

    #[test]
    fn resolve_index_invalid_ids() -> io::Result<()> {
        let file = make_test_file("id-invalid");
        save_to(&file, &named("Netflix", 9.99))?;

        let cases = [
            ("@0", io::ErrorKind::NotFound),  // zero is not a valid 1-based id
            ("@99", io::ErrorKind::NotFound), // out of bounds
            ("@abc", io::ErrorKind::InvalidInput), // non-numeric
        ];

        for (input, expected) in cases {
            let err = remove_from(&file, &[input]).expect_err("invalid id should fail");
            assert_eq!(err.kind(), expected, "input: {input}");
        }
        Ok(())
    }

    #[test]
    fn update_rejects_rename_to_existing_name() -> io::Result<()> {
        let file = make_test_file("rename-conflict");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        let err = update_from(&file, "Netflix", Some("spotify"), &Expense::default())
            .expect_err("rename conflict should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        Ok(())
    }

    #[test]
    fn remove_many_by_name() -> io::Result<()> {
        let file = make_test_file("remove-many-name");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        save_to(&file, &named("Rent", 999.0))?;
        let names = remove_from(&file, &["Netflix", "Rent"])?;
        assert_eq!(names, vec!["Netflix", "Rent"]);
        let remaining = list_from(&file)?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn remove_many_by_id_reverse_order() -> io::Result<()> {
        let file = make_test_file("remove-many-id");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        save_to(&file, &named("Rent", 999.0))?;
        // @3 then @1 — internal reverse order must not corrupt indices
        let names = remove_from(&file, &["@3", "@1"])?;
        assert_eq!(names, vec!["Rent", "Netflix"]);
        let remaining = list_from(&file)?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn remove_many_duplicate_target_returns_error() -> io::Result<()> {
        let file = make_test_file("remove-many-dup");
        save_to(&file, &named("Netflix", 9.99))?;
        let err = remove_from(&file, &["Netflix", "Netflix"]).expect_err("duplicate should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn remove_many_single_behaves_like_remove() -> io::Result<()> {
        let file = make_test_file("remove-many-single");
        save_to(&file, &named("Netflix", 9.99))?;
        save_to(&file, &named("Spotify", 5.99))?;
        let names = remove_from(&file, &["Netflix"])?;
        assert_eq!(names, vec!["Netflix"]);
        assert_eq!(list_from(&file)?.len(), 1);
        Ok(())
    }

    #[test]
    fn clear_category_no_match_returns_zero_and_no_snapshot() -> io::Result<()> {
        let file = make_test_file("clear-no-match");
        save_to(
            &file,
            &Expense {
                category: Some("streaming".into()),
                ..named("Netflix", 9.99)
            },
        )?;
        assert_eq!(clear_category_from(&file, "housing")?, 0);
        // no snapshot means undo is unavailable
        let err = restore_from(&file).expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn categories_from_empty_file_returns_empty() -> io::Result<()> {
        let file = make_test_file("categories-empty");
        assert!(categories_from(&file)?.is_empty());
        Ok(())
    }

    #[test]
    fn clear_category_from_removes_category_from_matching_expenses() -> io::Result<()> {
        let file = make_test_file("clear-category");
        save_to(
            &file,
            &Expense {
                category: Some("streaming".into()),
                ..named("Netflix", 9.99)
            },
        )?;
        save_to(
            &file,
            &Expense {
                category: Some("Streaming".into()),
                ..named("Spotify", 5.99)
            },
        )?;
        save_to(
            &file,
            &Expense {
                category: Some("housing".into()),
                ..named("Rent", 999.0)
            },
        )?;

        assert_eq!(clear_category_from(&file, "streaming")?, 2);

        let expenses = list_from(&file)?;
        assert_eq!(expenses[0].category, None);
        assert_eq!(expenses[1].category, None);
        assert_eq!(expenses[2].category.as_deref(), Some("housing"));
        Ok(())
    }
}
