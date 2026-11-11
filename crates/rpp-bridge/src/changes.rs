//! Telling which presences saw their pages change, so their scripts read at
//! once instead of at their next interval.

use std::collections::BTreeMap;
use std::time::Duration;

use tokio::sync::watch;

use crate::protocol::Page;

/// Wait after a change for the ones that come with it: a jump in a video fires
/// several media events, and they deserve one read.
const SETTLE: Duration = Duration::from_millis(200);

/// What identifies the pages of one presence: how many, and when they arrived.
type Mark = (usize, u64);

fn marks(pages: &[Page]) -> BTreeMap<&str, Mark> {
    let mut all: BTreeMap<&str, Mark> = BTreeMap::new();
    for page in pages {
        let mark = all.entry(page.presence.as_str()).or_default();
        mark.0 += 1;
        mark.1 = mark.1.wrapping_add(page.received_at);
    }
    all
}

/// The presences whose pages differ between two sets of marks.
fn changed(before: &BTreeMap<String, Mark>, after: &BTreeMap<&str, Mark>) -> Vec<String> {
    let gone = before
        .keys()
        .filter(|name| !after.contains_key(name.as_str()));
    let new_or_moved = after
        .iter()
        .filter(|(name, mark)| before.get(**name) != Some(mark))
        .map(|(name, _)| *name);
    gone.map(String::as_str)
        .chain(new_or_moved)
        .map(str::to_owned)
        .collect()
}

/// Calls `wake` with each presence whose pages change, until the bridge ends.
pub(crate) async fn follow(mut pages: watch::Receiver<Vec<Page>>, wake: impl Fn(&str)) {
    let mut seen = BTreeMap::new();
    while pages.changed().await.is_ok() {
        tokio::time::sleep(SETTLE).await;
        let names = {
            let current = pages.borrow_and_update();
            let after = marks(&current);
            let names = changed(&seen, &after);
            seen = after
                .into_iter()
                .map(|(name, mark)| (name.to_owned(), mark))
                .collect();
            names
        };
        for name in names {
            wake(&name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(presence: &str, tab: i64, received_at: u64) -> Page {
        Page {
            browser: "chrome".into(),
            tab,
            presence: presence.into(),
            url: "https://a.io/".into(),
            title: String::new(),
            values: Default::default(),
            received_at,
            visible: false,
            focused: false,
            focused_at: 0,
        }
    }

    fn seen(pages: &[Page]) -> BTreeMap<String, Mark> {
        marks(pages)
            .into_iter()
            .map(|(name, mark)| (name.to_owned(), mark))
            .collect()
    }

    #[test]
    fn only_the_presences_whose_pages_moved_are_named() {
        let before = [page("A", 1, 10), page("B", 2, 20)];
        let same = marks(&before);
        assert!(changed(&seen(&before), &same).is_empty());

        let updated = [page("A", 1, 11), page("B", 2, 20)];
        assert_eq!(changed(&seen(&before), &marks(&updated)), ["A"]);

        let closed = [page("B", 2, 20)];
        assert_eq!(changed(&seen(&before), &marks(&closed)), ["A"]);

        let opened = [page("A", 1, 10), page("B", 2, 20), page("C", 3, 30)];
        assert_eq!(changed(&seen(&before), &marks(&opened)), ["C"]);

        // One of two tabs of a presence closing counts.
        let two = [page("A", 1, 10), page("A", 2, 10)];
        let one = [page("A", 1, 10)];
        assert_eq!(changed(&seen(&two), &marks(&one)), ["A"]);
    }

    #[tokio::test]
    async fn bursts_wake_a_presence_once_and_the_end_of_the_bridge_ends_it() {
        use std::sync::{Arc, Mutex};

        let (sender, receiver) = watch::channel(Vec::new());
        let woken = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&woken);
        let task = tokio::spawn(follow(receiver, move |name| {
            log.lock().unwrap().push(name.to_owned());
        }));
        for at in 1..=5 {
            sender.send_replace(vec![page("A", 1, at)]);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        tokio::time::sleep(SETTLE * 2).await;
        assert_eq!(*woken.lock().unwrap(), ["A"]);
        drop(sender);
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
    }
}
