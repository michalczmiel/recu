use std::io;
use std::path::Path;

use chrono::NaiveDate;

use crate::expense::{self, Expense};

/// CSV columns we own. Anything else is round-tripped via `Expense::extra`.
const KNOWN_COLUMNS: &[&str] = &[
    "id",
    "name",
    "amount",
    "currency",
    "start_date",
    "interval",
    "category",
    "end_date",
];

/// Read every expense from the CSV at `path`. A missing file yields an empty list.
pub fn read_rows(path: &Path) -> io::Result<Vec<Expense>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    // `flexible(true)` so hand-edited CSVs with trailing-empty cells
    // omitted (common in spreadsheets) still parse.
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(io_invalid_data)?;
    let headers: Vec<String> = reader
        .headers()
        .map_err(io_invalid_data)?
        .iter()
        .map(str::to_string)
        .collect();
    let mut out = Vec::new();
    for record in reader.records() {
        let record = record.map_err(io_invalid_data)?;
        out.push(row_to_expense(&headers, &record)?);
    }
    Ok(out)
}

/// Write every expense to the CSV at `path`, overwriting it.
pub fn write_rows(path: &Path, entries: &[Expense]) -> io::Result<()> {
    let extra_keys = expense::extra_key_union(entries);

    let mut writer = csv::Writer::from_path(path).map_err(io_invalid_data)?;
    let mut header: Vec<&str> = KNOWN_COLUMNS.to_vec();
    header.extend(extra_keys.iter().map(String::as_str));
    writer.write_record(&header).map_err(io_invalid_data)?;
    for entry in entries {
        // Destructure exhaustively so adding a field to `Expense` forces
        // a corresponding update to `KNOWN_COLUMNS` and this row builder.
        let Expense {
            id,
            name,
            amount,
            currency,
            start_date,
            interval,
            category,
            end_date,
            extra,
        } = entry;
        let mut row: Vec<String> = vec![
            id.to_string(),
            name.clone(),
            amount.map(|a| a.to_string()).unwrap_or_default(),
            currency.clone().unwrap_or_default(),
            start_date.map(|d| d.to_string()).unwrap_or_default(),
            interval
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            category.clone().unwrap_or_default(),
            end_date.map(|d| d.to_string()).unwrap_or_default(),
        ];
        for k in &extra_keys {
            row.push(extra.get(k).cloned().unwrap_or_default());
        }
        writer.write_record(&row).map_err(io_invalid_data)?;
    }
    writer.flush()?;
    Ok(())
}

fn io_invalid_data<E: std::error::Error + Send + Sync + 'static>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

fn row_to_expense(headers: &[String], record: &csv::StringRecord) -> io::Result<Expense> {
    let mut e = Expense::default();
    for (h, v) in headers.iter().zip(record.iter()) {
        match h.as_str() {
            "id" => {
                e.id = v.parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("invalid id '{v}'"))
                })?;
            }
            "name" => e.name = v.to_string(),
            "amount" => {
                e.amount = if v.is_empty() {
                    None
                } else {
                    Some(v.parse::<f64>().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("invalid amount '{v}'"))
                    })?)
                };
            }
            "currency" => e.currency = empty_to_none(v),
            "start_date" => e.start_date = parse_opt_date(v)?,
            "interval" => {
                e.interval =
                    if v.is_empty() {
                        None
                    } else {
                        Some(v.parse().map_err(|msg: String| {
                            io::Error::new(io::ErrorKind::InvalidData, msg)
                        })?)
                    };
            }
            "category" => e.category = empty_to_none(v),
            "end_date" => e.end_date = parse_opt_date(v)?,
            other => {
                e.extra.insert(other.to_string(), v.to_string());
            }
        }
    }
    Ok(e)
}

fn empty_to_none(v: &str) -> Option<String> {
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn parse_opt_date(v: &str) -> io::Result<Option<NaiveDate>> {
    if v.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(v, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid date '{v}', expected YYYY-MM-DD"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use tempfile::TempDir;

    fn tmp_csv(contents: &str) -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("create tempdir");
        let path = dir.path().join("recu.csv");
        fs::write(&path, contents).expect("write csv");
        (dir, path)
    }

    #[test]
    fn read_missing_file_returns_empty() -> io::Result<()> {
        let dir = TempDir::new().expect("create tempdir");
        assert!(read_rows(&dir.path().join("nope.csv"))?.is_empty());
        Ok(())
    }

    #[test]
    fn loads_legacy_csv_without_end_date_column() -> io::Result<()> {
        let (_dir, path) = tmp_csv(
            "id,name,amount,currency,start_date,interval,category\n\
             1,Netflix,9.99,usd,,monthly,streaming\n",
        );
        let entries = read_rows(&path)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].end_date, None);
        assert_eq!(entries[0].category.as_deref(), Some("streaming"));
        Ok(())
    }

    #[test]
    fn flexible_short_row_loads_with_missing_trailing_cells() -> io::Result<()> {
        // Trailing empty cells omitted — common when spreadsheets export.
        let (_dir, path) = tmp_csv(
            "id,name,amount,currency,start_date,interval,category,end_date\n\
             1,Netflix,9.99,usd,,monthly\n",
        );
        let entries = read_rows(&path)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, None);
        assert_eq!(entries[0].end_date, None);
        Ok(())
    }

    #[test]
    fn empty_optional_cells_load_as_none() -> io::Result<()> {
        let (_dir, path) = tmp_csv(
            "id,name,amount,currency,start_date,interval,category,end_date\n\
             1,Netflix,,,,,,\n",
        );
        let e = &read_rows(&path)?[0];
        assert_eq!(e.amount, None);
        assert_eq!(e.currency, None);
        assert_eq!(e.start_date, None);
        assert_eq!(e.interval, None);
        assert_eq!(e.category, None);
        assert_eq!(e.end_date, None);
        Ok(())
    }

    #[test]
    fn invalid_amount_returns_invalid_data() {
        let (_dir, path) = tmp_csv(
            "id,name,amount,currency,start_date,interval,category,end_date\n\
             1,Netflix,not-a-number,usd,,monthly,,\n",
        );
        let err = read_rows(&path).expect_err("bad amount should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn round_trips_unknown_columns() -> io::Result<()> {
        let (_dir, path) = tmp_csv(
            "id,notes,name,vendor,amount,currency,start_date,interval,category,end_date\n\
             1,personal,Netflix,acme,9.99,usd,,monthly,,\n",
        );
        let entries = read_rows(&path)?;
        write_rows(&path, &entries)?;
        let raw = fs::read_to_string(&path)?;
        assert!(raw.contains("notes"), "notes header lost: {raw}");
        assert!(raw.contains("vendor"), "vendor header lost: {raw}");
        assert!(raw.contains("personal"), "notes value lost: {raw}");
        assert!(raw.contains("acme"), "vendor value lost: {raw}");
        Ok(())
    }
}
