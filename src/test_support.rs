#![allow(dead_code)]

use std::ops::Deref;

use chrono::NaiveDate;
use tempfile::TempDir;

use crate::expense::{Expense, Interval};
use crate::store::Store;

pub struct TestStore {
    _dir: TempDir,
    store: Store,
}

impl Deref for TestStore {
    type Target = Store;
    fn deref(&self) -> &Store {
        &self.store
    }
}

pub fn store() -> TestStore {
    let dir = TempDir::new().expect("create tempdir");
    let store = Store::at(dir.path().join("recu.csv"));
    TestStore { _dir: dir, store }
}

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

pub struct ExpenseBuilder {
    inner: Expense,
}

pub fn expense(name: &str) -> ExpenseBuilder {
    ExpenseBuilder {
        inner: Expense {
            name: name.to_string(),
            ..Default::default()
        },
    }
}

impl ExpenseBuilder {
    pub fn amount(mut self, a: f64) -> Self {
        self.inner.amount = Some(a);
        self
    }
    pub fn currency(mut self, c: &str) -> Self {
        self.inner.currency = Some(c.into());
        self
    }
    pub fn start(mut self, d: NaiveDate) -> Self {
        self.inner.start_date = Some(d);
        self
    }
    pub fn end(mut self, d: NaiveDate) -> Self {
        self.inner.end_date = Some(d);
        self
    }
    pub fn category(mut self, c: &str) -> Self {
        self.inner.category = Some(c.into());
        self
    }
    pub fn interval(mut self, i: Interval) -> Self {
        self.inner.interval = Some(i);
        self
    }
    pub fn weekly(self) -> Self {
        self.interval(Interval::Weekly)
    }
    pub fn monthly(self) -> Self {
        self.interval(Interval::Monthly)
    }
    pub fn quarterly(self) -> Self {
        self.interval(Interval::Quarterly)
    }
    pub fn yearly(self) -> Self {
        self.interval(Interval::Yearly)
    }
    pub fn build(self) -> Expense {
        self.inner
    }
}

pub fn snapshot_no_ansi() -> insta::internals::SettingsBindDropGuard {
    let mut s = insta::Settings::clone_current();
    s.add_filter(r"\x1b\[[0-9;]*m", "");
    s.bind_to_scope()
}
