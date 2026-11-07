//! What a script prints with `console.log`, kept for the developer tools.

use std::collections::VecDeque;

/// Lines kept until someone takes them.
const KEPT: usize = 200;
/// Lines accepted from one read, so a loop that prints cannot flood the buffer.
const PER_POLL: usize = 50;
/// Longest line, in characters.
const MAX_LINE: usize = 500;

/// A bounded buffer of printed lines.
#[derive(Debug, Default)]
pub(crate) struct Console {
    lines: VecDeque<String>,
    this_poll: usize,
}

impl Console {
    /// Starts counting the lines of a new read.
    pub(crate) fn begin_poll(&mut self) {
        self.this_poll = 0;
    }

    /// Adds a line; `level` is `log`, `warn` or `error`.
    pub(crate) fn push(&mut self, level: &str, text: &str) {
        if self.this_poll >= PER_POLL {
            return;
        }
        self.this_poll += 1;
        let text: String = text.chars().take(MAX_LINE).collect();
        let line = match level {
            "warn" | "error" => format!("[{level}] {text}"),
            _ => text,
        };
        if self.lines.len() == KEPT {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Hands over the kept lines, oldest first, and empties the buffer.
    pub(crate) fn take(&mut self) -> Vec<String> {
        self.lines.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_is_limited_and_long_lines_are_cut() {
        let mut console = Console::default();
        console.begin_poll();
        for number in 0..60 {
            console.push("log", &format!("line {number}"));
        }
        assert_eq!(console.take().len(), PER_POLL);
        console.begin_poll();
        console.push("log", &"x".repeat(900));
        assert_eq!(console.take()[0].chars().count(), MAX_LINE);
    }

    #[test]
    fn only_the_newest_lines_are_kept_and_levels_are_marked() {
        let mut console = Console::default();
        for round in 0..6 {
            console.begin_poll();
            for number in 0..50 {
                console.push("log", &format!("{round}-{number}"));
            }
        }
        let lines = console.take();
        assert_eq!(lines.len(), KEPT);
        assert_eq!(lines.last().map(String::as_str), Some("5-49"));

        console.begin_poll();
        console.push("warn", "careful");
        console.push("error", "broken");
        assert_eq!(console.take(), ["[warn] careful", "[error] broken"]);
        assert!(console.take().is_empty());
    }
}
