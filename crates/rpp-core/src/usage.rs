//! Time each presence spent on display, by day.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Days kept; each new day past this drops the oldest.
pub const KEPT_DAYS: usize = 56;
/// Most presences counted in one day. A real day is far below it, and a broken
/// script with a new name each second must not grow the file.
const MAX_NAMES: usize = 256;

/// Seconds on display, by day and by presence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Usage {
    days: BTreeMap<String, BTreeMap<String, u64>>,
}

impl Usage {
    /// Adds `seconds` on display to a presence on a `YYYY-MM-DD` day.
    pub fn add(&mut self, day: &str, name: &str, seconds: u64) {
        let names = self.days.entry(day.to_owned()).or_default();
        if let Some(total) = names.get_mut(name) {
            *total = total.saturating_add(seconds);
        } else if names.len() < MAX_NAMES {
            names.insert(name.to_owned(), seconds);
        }
        while self.days.len() > KEPT_DAYS {
            self.days.pop_first();
        }
    }

    /// The days counted, oldest first.
    pub fn days(&self) -> &BTreeMap<String, BTreeMap<String, u64>> {
        &self.days
    }

    /// The time as CSV: a `date,presence,seconds` header and one line per entry.
    /// Names that start with `=`, `+`, `-` or `@` get a leading quote, so a
    /// spreadsheet does not run them.
    pub fn csv(&self) -> String {
        let mut text = String::from("date,presence,seconds\n");
        for (day, names) in &self.days {
            for (name, seconds) in names {
                let plain = if name.starts_with(['=', '+', '-', '@']) {
                    format!("'{name}")
                } else {
                    name.clone()
                };
                let quoted = plain.replace('"', "\"\"");
                text.push_str(&format!("{day},\"{quoted}\",{seconds}\n"));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_adds_up_by_day_and_by_presence() {
        let mut usage = Usage::default();
        usage.add("2026-11-05", "YouTube", 5);
        usage.add("2026-11-05", "YouTube", 10);
        usage.add("2026-11-05", "Roblox", 1);
        usage.add("2026-11-06", "YouTube", 2);
        assert_eq!(usage.days()["2026-11-05"]["YouTube"], 15);
        assert_eq!(usage.days()["2026-11-05"]["Roblox"], 1);
        assert_eq!(usage.days()["2026-11-06"]["YouTube"], 2);
    }

    #[test]
    fn the_oldest_days_go_first() {
        let mut usage = Usage::default();
        for day in 1..=KEPT_DAYS + 3 {
            usage.add(
                &format!("2026-{:02}-{:02}", day / 28 + 1, day % 28 + 1),
                "A",
                1,
            );
        }
        assert_eq!(usage.days().len(), KEPT_DAYS);
        assert!(!usage.days().contains_key("2026-01-02"));
    }

    #[test]
    fn a_script_that_changes_its_name_cannot_grow_a_day_for_good() {
        let mut usage = Usage::default();
        for number in 0..MAX_NAMES * 2 {
            usage.add("2026-11-05", &format!("name {number}"), 1);
        }
        assert_eq!(usage.days()["2026-11-05"].len(), MAX_NAMES);
    }

    #[test]
    fn a_counter_never_wraps() {
        let mut usage = Usage::default();
        usage.add("2026-11-05", "A", u64::MAX);
        usage.add("2026-11-05", "A", 5);
        assert_eq!(usage.days()["2026-11-05"]["A"], u64::MAX);
    }

    #[test]
    fn the_table_cannot_run_as_a_formula() {
        let mut usage = Usage::default();
        usage.add("2026-11-05", "=HYPERLINK(\"x\")", 5);
        usage.add("2026-11-05", "YouTube", 7);
        assert_eq!(
            usage.csv(),
            "date,presence,seconds\n\
             2026-11-05,\"'=HYPERLINK(\"\"x\"\")\",5\n\
             2026-11-05,\"YouTube\",7\n"
        );
    }
}
