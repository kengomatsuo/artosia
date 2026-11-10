//! Small text helpers: query cleaning, comparison keys and lyric tidying.

/// Collapses whitespace and cuts to `max` characters.
pub(crate) fn clean_query(query: &str, max: usize) -> String {
    let joined = query.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(max).collect()
}

/// Comparison key: lowercase letters and digits, apostrophes and hyphens dropped, one space
/// between words. "Ku di tangan-Mu" and "ku di tanganmu" give the same key.
pub(crate) fn norm(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_space = false;
    for ch in value.chars() {
        if matches!(ch, '\'' | '\u{2019}' | '-') {
            continue;
        }
        if ch.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.extend(ch.to_lowercase());
        } else {
            pending_space = true;
        }
    }
    out
}

/// Removes `[mm:ss.xx]` stamps at the start of each line and `<mm:ss.xx>` word stamps anywhere.
pub(crate) fn strip_timestamps(synced: &str) -> String {
    synced.replace("\r\n", "\n").lines().map(strip_line).collect::<Vec<_>>().join("\n")
}

fn strip_line(line: &str) -> String {
    let mut rest = line.trim_start();
    while let Some(after) = strip_stamp(rest, '[', ']') {
        rest = after.trim_start();
    }
    let mut out = String::with_capacity(rest.len());
    let mut at = 0;
    while at < rest.len() {
        let tail = &rest[at..];
        if let Some(after) = strip_stamp(tail, '<', '>') {
            at += tail.len() - after.len();
            continue;
        }
        let Some(ch) = tail.chars().next() else { break };
        out.push(ch);
        at += ch.len_utf8();
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `[01:23.45]` or `<1:23>` at the start of `text`; returns what follows.
fn strip_stamp(text: &str, open: char, close: char) -> Option<&str> {
    let inner = text.strip_prefix(open)?;
    let end = inner.find(close)?;
    let mut parts = inner[..end].split(':');
    let minutes = parts.next()?;
    let seconds = parts.next()?;
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.');
    (parts.next().is_none() && digits(minutes) && digits(seconds)).then(|| &inner[end + close.len_utf8()..])
}

/// Unix newlines, no trailing spaces, at most one blank line between stanzas, no blank edges.
pub(crate) fn tidy(raw: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let unix = raw.replace("\r\n", "\n").replace('\r', "\n");
    for line in unix.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            if out.last().is_some_and(|last| !last.is_empty()) {
                out.push("");
            }
        } else {
            out.push(line);
        }
    }
    while out.last().is_some_and(|last| last.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_ignores_case_punctuation_and_apostrophes() {
        assert_eq!(norm("  Ku di tangan-Mu! "), "ku di tanganmu");
        assert_eq!(norm("T'rima  Sukacita"), "trima sukacita");
    }

    #[test]
    fn query_is_collapsed_and_cut() {
        assert_eq!(clean_query("  a \n b\t c ", 10), "a b c");
        assert_eq!(clean_query("abcdef", 3), "abc");
    }

    #[test]
    fn stamps_are_stripped() {
        let synced = "[00:12.34] One <00:12.50> two\n[01:02] Three\n[00:03.00]\n[ar: Someone]";
        assert_eq!(strip_timestamps(synced), "One two\nThree\n\n[ar: Someone]");
    }

    #[test]
    fn tidy_collapses_blank_runs() {
        assert_eq!(tidy("\n\nA  \nB\n\n\n\nC\n\n"), "A\nB\n\nC");
    }
}
