//! Tidies recogniser output before the operator sees it.

/// Removes bracketed annotations such as `[BLANK_AUDIO]` or `(music)`, music notes and
/// asterisk sounds, collapses a word repeated many times in a row (a decoder loop), and
/// squeezes spaces. Returns `None` when nothing worth searching for is left.
pub fn clean_text(raw: &str) -> Option<String> {
    let mut kept = String::with_capacity(raw.len());
    let mut depth_square = 0usize;
    let mut depth_round = 0usize;
    let mut in_star = false;
    for ch in raw.chars() {
        match ch {
            '[' => depth_square += 1,
            ']' => depth_square = depth_square.saturating_sub(1),
            '(' => depth_round += 1,
            ')' => depth_round = depth_round.saturating_sub(1),
            '*' => in_star = !in_star,
            '\u{266a}' | '\u{266b}' | '\u{266c}' => kept.push(' '),
            _ if depth_square > 0 || depth_round > 0 || in_star => {}
            _ => kept.push(ch),
        }
    }
    let words = collapse_repeats(kept.split_whitespace().collect(), 3);
    let text = words.join(" ");
    text.chars().any(char::is_alphanumeric).then_some(text)
}

/// Keeps at most `max_run` identical words (case-insensitive, ignoring edge punctuation) in a row.
fn collapse_repeats(words: Vec<&str>, max_run: usize) -> Vec<&str> {
    let norm = |w: &str| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let mut out: Vec<&str> = Vec::with_capacity(words.len());
    let mut run = 0usize;
    for w in words {
        let same = out.last().is_some_and(|prev| norm(prev) == norm(w));
        run = if same { run + 1 } else { 1 };
        if run <= max_run {
            out.push(w);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_are_removed() {
        assert_eq!(clean_text(" [BLANK_AUDIO] "), None);
        assert_eq!(clean_text("(music)"), None);
        assert_eq!(clean_text("*applause*"), None);
        assert_eq!(clean_text("\u{266a} \u{266a}"), None);
        assert_eq!(
            clean_text("Yohanes tiga (batuk) ayat enam belas").as_deref(),
            Some("Yohanes tiga ayat enam belas")
        );
    }

    #[test]
    fn loops_are_collapsed() {
        assert_eq!(clean_text("ya ya ya ya ya ya ya amin").as_deref(), Some("ya ya ya amin"));
        assert_eq!(clean_text("Tuhan, tuhan tuhan").as_deref(), Some("Tuhan, tuhan tuhan"));
    }

    #[test]
    fn punctuation_only_is_nothing() {
        assert_eq!(clean_text("... !"), None);
        assert_eq!(clean_text(""), None);
    }

    #[test]
    fn indonesian_text_survives() {
        assert_eq!(
            clean_text("Bagi Bapa yang mahakuasa").as_deref(),
            Some("Bagi Bapa yang mahakuasa")
        );
    }
}
