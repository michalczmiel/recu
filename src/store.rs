use std::fs;
use std::io;
use std::path::PathBuf;

use crate::expense::Expense;

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn undo_path(&self) -> PathBuf {
        self.path.with_extension("csv.undo")
    }

    fn seq_path(&self) -> PathBuf {
        self.path.with_extension("csv.seq")
    }

    fn snapshot(&self) -> io::Result<()> {
        if self.path.exists() {
            fs::copy(&self.path, self.undo_path())?;
        }
        Ok(())
    }

    fn read_seq(&self) -> Option<u64> {
        fs::read_to_string(self.seq_path())
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// Next id to assign. Monotonic and never reused — even after delete or undo —
    /// because we only ever advance the persisted seq, never roll it back.
    fn next_id(&self, entries: &[Expense]) -> u64 {
        let max_existing = entries.iter().map(|e| e.id).max().unwrap_or(0);
        self.read_seq().unwrap_or(0).max(max_existing + 1)
    }

    pub fn list(&self) -> io::Result<Vec<Expense>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let mut reader = csv::Reader::from_path(&self.path).map_err(io_invalid_data)?;
        reader
            .deserialize()
            .collect::<Result<Vec<_>, _>>()
            .map_err(io_invalid_data)
    }

    fn write_all(&self, entries: &[Expense]) -> io::Result<()> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("csv.tmp");
        {
            let mut writer = csv::Writer::from_path(&tmp).map_err(io_invalid_data)?;
            for entry in entries {
                writer.serialize(entry).map_err(io_invalid_data)?;
            }
            writer.flush()?;
        }
        fs::rename(tmp, &self.path)?;

        let max_id = entries.iter().map(|e| e.id).max().unwrap_or(0);
        let needed_seq = max_id + 1;
        if needed_seq > self.read_seq().unwrap_or(0) {
            fs::write(self.seq_path(), needed_seq.to_string())?;
        }
        Ok(())
    }

    pub fn save(&self, expense: &Expense) -> io::Result<()> {
        self.snapshot()?;
        let mut entries = self.list()?;
        if entries
            .iter()
            .any(|e| e.name.eq_ignore_ascii_case(&expense.name))
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("expense '{}' already exists", expense.name),
            ));
        }
        let mut new_entry = expense.clone();
        new_entry.id = self.next_id(&entries);
        entries.push(new_entry);
        self.write_all(&entries)
    }

    pub fn get(&self, target: &str) -> io::Result<Expense> {
        let mut entries = self.list()?;
        let index = resolve_index_in(&entries, target)?;
        Ok(entries.swap_remove(index))
    }

    pub fn update(&self, target: &str, changes: &Expense) -> io::Result<()> {
        let mut entries = self.list()?;
        let index = resolve_index_in(&entries, target)?;

        if changes.amount.is_none()
            && changes.currency.is_none()
            && changes.start_date.is_none()
            && changes.interval.is_none()
            && changes.category.is_none()
            && changes.end_date.is_none()
        {
            return Ok(());
        }

        self.snapshot()?;

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
        expense.end_date = changes.end_date.or(expense.end_date);

        self.write_all(&entries)
    }

    pub fn rename(&self, target: &str, new_name: &str) -> io::Result<()> {
        let mut entries = self.list()?;
        let index = resolve_index_in(&entries, target)?;

        if entries
            .iter()
            .enumerate()
            .any(|(i, e)| i != index && e.name.eq_ignore_ascii_case(new_name))
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("expense '{new_name}' already exists"),
            ));
        }

        self.snapshot()?;
        entries[index].name = new_name.to_string();
        self.write_all(&entries)
    }

    pub fn remove(&self, targets: &[&str]) -> io::Result<Vec<String>> {
        self.snapshot()?;
        let mut entries = self.list()?;

        let mut resolved: Vec<(usize, usize)> = Vec::with_capacity(targets.len());
        for (pos, target) in targets.iter().enumerate() {
            let index = resolve_index_in(&entries, target)?;
            if resolved.iter().any(|&(_, i)| i == index) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("duplicate target '{target}'"),
                ));
            }
            resolved.push((pos, index));
        }

        // Remove from highest index first so earlier removals don't shift remaining ones.
        resolved.sort_unstable_by_key(|&(_, idx)| std::cmp::Reverse(idx));

        let mut names = vec![String::new(); resolved.len()];
        for (pos, idx) in resolved {
            names[pos] = entries.remove(idx).name;
        }

        self.write_all(&entries)?;
        Ok(names)
    }

    pub fn categories(&self) -> io::Result<Vec<String>> {
        let mut seen = std::collections::HashSet::<String>::new();
        let mut categories = Vec::<String>::new();
        for expense in self.list()? {
            let Some(category) = expense.category else {
                continue;
            };
            if seen.insert(category.to_ascii_lowercase()) {
                categories.push(category);
            }
        }
        categories.sort_by_cached_key(|c| c.to_ascii_lowercase());
        Ok(categories)
    }

    pub fn reassign_category(&self, sources: &[&str], dst: &str) -> io::Result<Vec<usize>> {
        let mut entries = self.list()?;
        let mut counts = vec![0usize; sources.len()];
        let mut changed = false;

        for entry in &mut entries {
            let Some(current) = entry.category.as_deref() else {
                continue;
            };
            if let Some(i) = sources.iter().position(|s| current.eq_ignore_ascii_case(s)) {
                counts[i] += 1;
                if current != dst {
                    entry.category = Some(dst.to_string());
                    changed = true;
                }
            }
        }

        if !changed {
            return Ok(counts);
        }

        self.snapshot()?;
        self.write_all(&entries)?;
        Ok(counts)
    }

    pub fn clear_categories(&self, categories: &[&str]) -> io::Result<Vec<usize>> {
        let mut entries = self.list()?;
        let mut counts = vec![0usize; categories.len()];

        for entry in &mut entries {
            let Some(current) = entry.category.as_deref() else {
                continue;
            };
            if let Some(i) = categories
                .iter()
                .position(|c| current.eq_ignore_ascii_case(c))
            {
                counts[i] += 1;
                entry.category = None;
            }
        }

        if counts.iter().all(|&n| n == 0) {
            return Ok(counts);
        }

        self.snapshot()?;
        self.write_all(&entries)?;
        Ok(counts)
    }

    pub fn restore(&self) -> io::Result<String> {
        let undo = self.undo_path();
        if !undo.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "nothing to undo"));
        }
        let before = Store::at(&undo).list()?;
        let after = self.list()?;
        let msg = diff_description(&before, &after);
        fs::rename(&undo, &self.path)?;
        Ok(msg)
    }
}

fn io_invalid_data<E: std::error::Error + Send + Sync + 'static>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

fn resolve_index_in(entries: &[Expense], target: &str) -> io::Result<usize> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: u64 = id_str.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid id '{target}'"),
            )
        })?;
        return entries.iter().position(|e| e.id == id).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no expense at @{id}. Run 'recu ls' to see available expenses"),
            )
        });
    }

    entries
        .iter()
        .position(|e| e.name.eq_ignore_ascii_case(target))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("expense '{target}' not found. Run 'recu ls' to see available expenses"),
            )
        })
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

    fn make_store(name: &str) -> Store {
        let file = std::env::temp_dir().join(format!("recu-test-{name}.csv"));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(file.with_extension("csv.undo"));
        let _ = fs::remove_file(file.with_extension("csv.seq"));
        Store::at(file)
    }

    #[test]
    fn save_rejects_duplicate_case_insensitive() -> io::Result<()> {
        let store = make_store("storage-dup");
        store.save(&named("Netflix", 9.99))?;
        let err = store
            .save(&named("netflix", 9.99))
            .expect_err("duplicate save should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(store.list()?.len(), 1);
        Ok(())
    }

    #[test]
    fn list_from_missing_file_returns_empty() -> io::Result<()> {
        let store = make_store("storage-missing");
        assert!(store.list()?.is_empty());
        Ok(())
    }

    #[test]
    fn restore_after_remove() -> io::Result<()> {
        let store = make_store("undo-remove");
        store.save(&named("Netflix", 9.99))?;
        store.remove(&["Netflix"])?;
        assert!(store.list()?.is_empty());
        let msg = store.restore()?;
        assert_eq!(msg, "Restored 'Netflix'");
        assert_eq!(store.list()?.len(), 1);
        Ok(())
    }

    #[test]
    fn restore_after_update() -> io::Result<()> {
        let store = make_store("undo-update");
        store.save(&named("Netflix", 9.99))?;
        store.update("Netflix", &named("Netflix", 14.99))?;
        assert_eq!(store.list()?[0].amount, Some(14.99));
        let msg = store.restore()?;
        assert_eq!(msg, "Reverted edit of 'Netflix'");
        assert_eq!(store.list()?[0].amount, Some(9.99));
        Ok(())
    }

    #[test]
    fn restore_after_add() -> io::Result<()> {
        let store = make_store("undo-add");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        assert_eq!(store.list()?.len(), 2);
        let msg = store.restore()?;
        assert_eq!(msg, "Undid add of 'Spotify'");
        assert_eq!(store.list()?.len(), 1);
        Ok(())
    }

    #[test]
    fn restore_with_no_snapshot_returns_error() -> io::Result<()> {
        let store = make_store("undo-nosnap");
        store.save(&named("Netflix", 9.99))?;
        let err = store
            .restore()
            .expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn restore_is_single_use() -> io::Result<()> {
        let store = make_store("undo-singleuse");
        store.save(&named("Netflix", 9.99))?;
        store.remove(&["Netflix"])?;
        store.restore()?;
        let err = store.restore().expect_err("second restore should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn empty_patch_preserves_prior_undo() -> io::Result<()> {
        let store = make_store("update-empty-preserves-undo");
        store.save(&named("Netflix", 9.99))?;
        store.update("Netflix", &named("Netflix", 14.99))?;
        // Empty patch should be a no-op that does not consume the undo snapshot.
        store.update("Netflix", &Expense::default())?;
        store.restore()?;
        assert_eq!(store.list()?[0].amount, Some(9.99));
        Ok(())
    }

    #[test]
    fn categories_lists_unique_case_insensitive() -> io::Result<()> {
        let store = make_store("categories");
        store.save(&Expense {
            category: Some("Streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Spotify", 5.99)
        })?;
        store.save(&Expense {
            category: Some("Housing".into()),
            ..named("Rent", 999.0)
        })?;
        assert_eq!(store.categories()?, vec!["Housing", "Streaming"]);
        Ok(())
    }

    #[test]
    fn remove_by_id() -> io::Result<()> {
        let store = make_store("remove-by-id");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        let names = store.remove(&["@1"])?;
        assert_eq!(names, vec!["Netflix"]);
        let entries = store.list()?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn update_by_id() -> io::Result<()> {
        let store = make_store("update-by-id");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        store.update("@2", &named("Spotify", 7.99))?;
        assert_eq!(store.list()?[1].amount, Some(7.99));
        Ok(())
    }

    #[test]
    fn resolve_index_invalid_ids() -> io::Result<()> {
        let store = make_store("id-invalid");
        store.save(&named("Netflix", 9.99))?;

        let cases = [
            ("@0", io::ErrorKind::NotFound),
            ("@99", io::ErrorKind::NotFound),
            ("@abc", io::ErrorKind::InvalidInput),
        ];

        for (input, expected) in cases {
            let err = store.remove(&[input]).expect_err("invalid id should fail");
            assert_eq!(err.kind(), expected, "input: {input}");
        }
        Ok(())
    }

    #[test]
    fn rename_rejects_existing_name() -> io::Result<()> {
        let store = make_store("rename-conflict");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        let err = store
            .rename("Netflix", "spotify")
            .expect_err("rename conflict should fail");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        Ok(())
    }

    #[test]
    fn remove_many_by_name() -> io::Result<()> {
        let store = make_store("remove-many-name");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        store.save(&named("Rent", 999.0))?;
        let names = store.remove(&["Netflix", "Rent"])?;
        assert_eq!(names, vec!["Netflix", "Rent"]);
        let remaining = store.list()?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn remove_many_by_id_reverse_order() -> io::Result<()> {
        let store = make_store("remove-many-id");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        store.save(&named("Rent", 999.0))?;
        let names = store.remove(&["@3", "@1"])?;
        assert_eq!(names, vec!["Rent", "Netflix"]);
        let remaining = store.list()?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Spotify");
        Ok(())
    }

    #[test]
    fn remove_many_duplicate_target_returns_error() -> io::Result<()> {
        let store = make_store("remove-many-dup");
        store.save(&named("Netflix", 9.99))?;
        let err = store
            .remove(&["Netflix", "Netflix"])
            .expect_err("duplicate should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn remove_many_single_behaves_like_remove() -> io::Result<()> {
        let store = make_store("remove-many-single");
        store.save(&named("Netflix", 9.99))?;
        store.save(&named("Spotify", 5.99))?;
        let names = store.remove(&["Netflix"])?;
        assert_eq!(names, vec!["Netflix"]);
        assert_eq!(store.list()?.len(), 1);
        Ok(())
    }

    #[test]
    fn clear_category_no_match_returns_zero_and_no_snapshot() -> io::Result<()> {
        let store = make_store("clear-no-match");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        assert_eq!(store.clear_categories(&["housing"])?, vec![0]);
        let err = store
            .restore()
            .expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn categories_empty_file_returns_empty() -> io::Result<()> {
        let store = make_store("categories-empty");
        assert!(store.categories()?.is_empty());
        Ok(())
    }

    #[test]
    fn clear_category_removes_matching_expenses() -> io::Result<()> {
        let store = make_store("clear-category");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.save(&Expense {
            category: Some("Streaming".into()),
            ..named("Spotify", 5.99)
        })?;
        store.save(&Expense {
            category: Some("housing".into()),
            ..named("Rent", 999.0)
        })?;

        assert_eq!(store.clear_categories(&["streaming"])?, vec![2]);

        let expenses = store.list()?;
        assert_eq!(expenses[0].category, None);
        assert_eq!(expenses[1].category, None);
        assert_eq!(expenses[2].category.as_deref(), Some("housing"));
        Ok(())
    }

    #[test]
    fn reassign_renames_matching_expenses() -> io::Result<()> {
        let store = make_store("reassign-rename");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.save(&Expense {
            category: Some("Streaming".into()),
            ..named("Spotify", 5.99)
        })?;
        store.save(&Expense {
            category: Some("housing".into()),
            ..named("Rent", 999.0)
        })?;

        assert_eq!(store.reassign_category(&["streaming"], "Subs")?, vec![2]);

        let expenses = store.list()?;
        assert_eq!(expenses[0].category.as_deref(), Some("Subs"));
        assert_eq!(expenses[1].category.as_deref(), Some("Subs"));
        assert_eq!(expenses[2].category.as_deref(), Some("housing"));
        Ok(())
    }

    #[test]
    fn reassign_merges_multiple_sources() -> io::Result<()> {
        let store = make_store("reassign-merge");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.save(&Expense {
            category: Some("subs".into()),
            ..named("Spotify", 5.99)
        })?;
        store.save(&Expense {
            category: Some("housing".into()),
            ..named("Rent", 999.0)
        })?;

        let counts = store.reassign_category(&["streaming", "subs"], "Subs")?;
        assert_eq!(counts, vec![1, 1]);

        let expenses = store.list()?;
        assert_eq!(expenses[0].category.as_deref(), Some("Subs"));
        assert_eq!(expenses[1].category.as_deref(), Some("Subs"));
        assert_eq!(expenses[2].category.as_deref(), Some("housing"));
        Ok(())
    }

    #[test]
    fn reassign_no_match_skips_snapshot() -> io::Result<()> {
        let store = make_store("reassign-nomatch");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        assert_eq!(store.reassign_category(&["housing"], "Home")?, vec![0]);
        let err = store
            .restore()
            .expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn reassign_same_casing_skips_snapshot() -> io::Result<()> {
        let store = make_store("reassign-same");
        store.save(&Expense {
            category: Some("Streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        // Source matches dst exactly — counted but no mutation, no snapshot.
        assert_eq!(
            store.reassign_category(&["Streaming"], "Streaming")?,
            vec![1]
        );
        let err = store
            .restore()
            .expect_err("restore without snapshot should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn reassign_supports_undo() -> io::Result<()> {
        let store = make_store("reassign-undo");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.reassign_category(&["streaming"], "Subs")?;
        assert_eq!(store.list()?[0].category.as_deref(), Some("Subs"));
        let msg = store.restore()?;
        assert_eq!(msg, "Reverted edit of 'Netflix'");
        assert_eq!(store.list()?[0].category.as_deref(), Some("streaming"));
        Ok(())
    }

    #[test]
    fn clear_categories_multiple() -> io::Result<()> {
        let store = make_store("clear-categories-multi");
        store.save(&Expense {
            category: Some("streaming".into()),
            ..named("Netflix", 9.99)
        })?;
        store.save(&Expense {
            category: Some("housing".into()),
            ..named("Rent", 999.0)
        })?;
        store.save(&Expense {
            category: Some("food".into()),
            ..named("Groceries", 50.0)
        })?;

        let counts = store.clear_categories(&["streaming", "housing"])?;
        assert_eq!(counts, vec![1, 1]);

        let expenses = store.list()?;
        assert_eq!(expenses[0].category, None);
        assert_eq!(expenses[1].category, None);
        assert_eq!(expenses[2].category.as_deref(), Some("food"));
        Ok(())
    }
}
