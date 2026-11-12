//! Text normalisation and typo-tolerant word matching shared by every search.

/// One word of a text: where it sits in the original and its folded form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub start: usize,
    pub end: usize,
    pub norm: String,
    /// The word with its sung elision written out: `s'gala` also reads `segala`.
    pub full: Option<String>,
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}

/// Folds a character to what people type: lowercase ASCII where a Latin letter has an accent.
fn fold(c: char) -> Option<char> {
    let c = match c {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => 'a',
        'Ç' | 'ç' | 'Ć' | 'ć' | 'Č' | 'č' => 'c',
        'È'..='Ë' | 'è'..='ë' | 'Ē' | 'ē' | 'Ę' | 'ę' | 'Ě' | 'ě' => 'e',
        'Ì'..='Ï' | 'ì'..='ï' | 'Ī' | 'ī' => 'i',
        'Ñ' | 'ñ' | 'Ń' | 'ń' => 'n',
        'Ò'..='Ö' | 'ò'..='ö' | 'Ø' | 'ø' | 'Ō' | 'ō' => 'o',
        'Ù'..='Ü' | 'ù'..='ü' | 'Ū' | 'ū' => 'u',
        'Ý' | 'ý' | 'ÿ' => 'y',
        'Š' | 'š' | 'Ś' | 'ś' => 's',
        'Ž' | 'ž' | 'Ź' | 'ź' | 'Ż' | 'ż' => 'z',
        'Ł' | 'ł' => 'l',
        'Đ' | 'đ' | 'Ď' | 'ď' => 'd',
        _ => c,
    };
    if c.is_alphanumeric() { c.to_lowercase().next() } else { None }
}

fn joins_word(c: char) -> bool {
    matches!(c, '\'' | '\u{2019}' | '\u{2018}' | '`')
}

fn is_consonant(c: char) -> bool {
    c.is_ascii_alphabetic() && !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Indonesian lyrics drop an `e` to fit the tune and mark it with an apostrophe between two
/// consonants: s'gala, t'lah, kar'na. Returns the word with those `e`s put back.
fn written_out(norm: &str, elisions: &[usize]) -> Option<String> {
    let chars: Vec<char> = norm.chars().collect();
    let mut out = String::with_capacity(norm.len() + elisions.len());
    let mut any = false;
    for (i, c) in chars.iter().enumerate() {
        if elisions.contains(&i) && i > 0 && is_consonant(chars[i - 1]) && is_consonant(*c) {
            out.push('e');
            any = true;
        }
        out.push(*c);
    }
    any.then_some(out)
}

/// Splits text into searchable words. Apostrophes vanish inside a word ("don't" is "dont"),
/// every other punctuation mark separates, and each CJK character is a word of its own.
pub fn words(text: &str) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut current: Option<Word> = None;
    // Character positions in the open word that follow an apostrophe.
    let mut elisions: Vec<usize> = Vec::new();
    let close = |word: Option<Word>, elisions: &mut Vec<usize>, out: &mut Vec<Word>| {
        if let Some(mut word) = word {
            word.full = written_out(&word.norm, elisions);
            out.push(word);
        }
        elisions.clear();
    };
    for (at, c) in text.char_indices() {
        let end = at + c.len_utf8();
        if is_cjk(c) {
            close(current.take(), &mut elisions, &mut out);
            out.push(Word { start: at, end, norm: c.to_string(), full: None });
        } else if let Some(folded) = fold(c) {
            match current.as_mut() {
                Some(word) => {
                    word.norm.push(folded);
                    word.end = end;
                }
                None => current = Some(Word { start: at, end, norm: folded.to_string(), full: None }),
            }
        } else if joins_word(c) && current.is_some() {
            // The apostrophe adds no character; it marks where a sung `e` may be missing.
            if let Some(word) = &current {
                elisions.push(word.norm.chars().count());
            }
        } else {
            close(current.take(), &mut elisions, &mut out);
        }
    }
    close(current.take(), &mut elisions, &mut out);
    out
}

/// The folded words of a text joined by single spaces.
/// Korean text for a box that wraps: a word joiner between the syllables of each word, so a
/// line breaks at a space and never inside a word. Korean readers take a word cut in two as
/// a mistake, and the text engine has no setting for it. Text with no Korean in it comes
/// back as it is.
pub fn keep_words(text: &str) -> std::borrow::Cow<'_, str> {
    let hangul = |c: char| ('\u{ac00}'..='\u{d7a3}').contains(&c);
    if !text.chars().any(hangul) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    let mut before: Option<char> = None;
    for c in text.chars() {
        // A number written against a word ("2절") stays with it too.
        if before.is_some_and(|b| (hangul(b) && (hangul(c) || c.is_ascii_digit())) || (b.is_ascii_digit() && hangul(c))) {
            out.push('\u{2060}');
        }
        out.push(c);
        before = Some(c);
    }
    std::borrow::Cow::Owned(out)
}

pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in words(text) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&word.norm);
    }
    out
}

/// Typos a word of this many characters may carry and still match.
pub fn typo_budget(chars: usize) -> usize {
    match chars {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// Edit distance counting a swap of two neighbours as one edit; None when above `max`.
pub fn edit_distance(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let width = b.len() + 1;
    let mut before: Vec<usize> = vec![0; width];
    let mut previous: Vec<usize> = (0..width).collect();
    let mut row: Vec<usize> = vec![0; width];
    for i in 1..=a.len() {
        row[0] = i;
        let mut best = row[0];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut value = (previous[j] + 1).min(row[j - 1] + 1).min(previous[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                value = value.min(before[j - 2] + 1);
            }
            row[j] = value;
            best = best.min(value);
        }
        if best > max {
            return None;
        }
        std::mem::swap(&mut before, &mut previous);
        std::mem::swap(&mut previous, &mut row);
    }
    let distance = previous[b.len()];
    (distance <= max).then_some(distance)
}

/// How a typed token matched a word. Search orders results by these facts one at a time,
/// after the owner's house-kit palette: fewer typos, then whole words, then word starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordMatch {
    /// Edits that turn the token into the word.
    pub typos: u8,
    /// The token is the whole word.
    pub exact: bool,
    /// The token sits inside the word and not at its start.
    pub infix: bool,
}

impl WordMatch {
    /// One number that orders matches as the cascade does; lower is better.
    pub fn rank(self) -> u8 {
        self.typos * 4 + if self.exact { 0 } else { 2 } + u8::from(self.infix)
    }
}

/// The fewest letters a token needs before it may match inside a word. Indonesian puts
/// prefixes on its roots, so "berkat" should find "diberkati".
pub const INFIX_MIN: usize = 3;

/// How a typed token matches a word, or None when it does not.
/// `typing` marks the last token of a query, which may be an unfinished word.
pub fn match_word(token: &str, word: &str, typing: bool) -> Option<WordMatch> {
    if token == word {
        return Some(WordMatch { typos: 0, exact: true, infix: false });
    }
    let token_chars = token.chars().count();
    let word_chars = word.chars().count();
    if word.starts_with(token) && (typing || token_chars >= 3) {
        return Some(WordMatch { typos: 0, exact: false, infix: false });
    }
    if token_chars >= INFIX_MIN && word_chars > token_chars && word.contains(token) {
        return Some(WordMatch { typos: 0, exact: false, infix: true });
    }
    let budget = typo_budget(token_chars);
    if budget == 0 {
        return None;
    }
    if let Some(distance) = edit_distance(token, word, budget) {
        return Some(WordMatch { typos: distance as u8, exact: false, infix: false });
    }
    if typing && token_chars >= 4 && word_chars > token_chars {
        let head: String = word.chars().take(token_chars).collect();
        if edit_distance(token, &head, 1).is_some() {
            return Some(WordMatch { typos: 1, exact: false, infix: false });
        }
    }
    None
}

/// How well a typed token matches a word, 0 for no match and 1 for the same word. Bible text
/// search blends these; a match inside a word does not count there.
pub fn match_quality(token: &str, word: &str, typing: bool) -> f32 {
    let Some(found) = match_word(token, word, typing) else { return 0.0 };
    if found.exact {
        return 1.0;
    }
    if found.infix {
        return 0.0;
    }
    match found.typos {
        0 => {
            let extra = word.chars().count().saturating_sub(token.chars().count()) as f32;
            (0.9 - extra * 0.03).max(0.62)
        }
        1 if word.chars().count() > token.chars().count() + 1 => 0.55,
        1 => 0.7,
        _ => 0.5,
    }
}

/// Byte ranges of the words in `text` that match any query token, for highlighting.
pub fn highlight_ranges(text: &str, tokens: &[String]) -> Vec<std::ops::Range<usize>> {
    let last = tokens.len().saturating_sub(1);
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for word in words(text) {
        let hit = tokens.iter().enumerate().any(|(i, token)| {
            match_word(token, &word.norm, i == last).is_some() || word.full.as_ref().is_some_and(|full| match_word(token, full, i == last).is_some())
        });
        if !hit {
            continue;
        }
        match ranges.last_mut() {
            // Joins neighbours split only by a space so a phrase highlights as one run.
            Some(previous) if text[previous.end..word.start].chars().all(|c| c == ' ') => previous.end = word.end,
            _ => ranges.push(word.start..word.end),
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    #[test]
    fn korean_words_stay_whole_and_other_text_is_untouched() {
        assert!(matches!(super::keep_words("Bread for the road"), std::borrow::Cow::Borrowed(_)));
        let kept = super::keep_words("\u{c608}\u{bc30} \u{c21c}\u{c11c} 2\u{c808}");
        assert_eq!(kept, "\u{c608}\u{2060}\u{bc30} \u{c21c}\u{2060}\u{c11c} 2\u{2060}\u{c808}");
        assert_eq!(kept.replace('\u{2060}', ""), "\u{c608}\u{bc30} \u{c21c}\u{c11c} 2\u{c808}");
    }

    use super::*;

    #[test]
    fn words_fold_case_accents_and_apostrophes() {
        let found: Vec<String> = words("Don't  STOP, Señor!").into_iter().map(|w| w.norm).collect();
        assert_eq!(found, ["dont", "stop", "senor"]);
    }

    #[test]
    fn words_keep_original_positions() {
        let text = "Oh, we've come";
        let found = words(text);
        assert_eq!(&text[found[1].start..found[1].end], "we've");
    }

    #[test]
    fn cjk_characters_are_single_words() {
        let found: Vec<String> = words("起初 神").into_iter().map(|w| w.norm).collect();
        assert_eq!(found, ["起", "初", "神"]);
    }

    #[test]
    fn edit_distance_counts_swaps_once() {
        assert_eq!(edit_distance("grace", "garce", 2), Some(1));
        assert_eq!(edit_distance("grace", "grace", 0), Some(0));
        assert_eq!(edit_distance("grace", "glory", 2), None);
    }

    #[test]
    fn short_words_allow_no_typos() {
        assert_eq!(match_quality("god", "gad", false), 0.0);
        assert!(match_quality("amazng", "amazing", false) > 0.0);
    }

    #[test]
    fn an_unfinished_last_word_matches_by_prefix() {
        assert!(match_quality("ama", "amazing", true) > 0.6);
        assert!(match_quality("am", "amazing", true) > 0.0);
        assert_eq!(match_quality("am", "amazing", false), 0.0);
    }

    #[test]
    fn matches_order_whole_word_then_start_then_inside_then_typo() {
        let rank = |token: &str, word: &str| match_word(token, word, false).map(WordMatch::rank);
        assert!(rank("berkat", "berkat") < rank("berkat", "berkatmu"));
        assert!(rank("berkat", "berkatmu") < rank("berkat", "diberkati"));
        assert!(rank("berkat", "diberkati") < rank("berkat", "berkas"));
        assert_eq!(rank("ku", "aku"), None, "two letters never match inside a word");
    }

    #[test]
    fn a_sung_elision_also_reads_written_out() {
        let found = words("s'gala puji kar'na Dia, don't");
        assert_eq!(found[0].norm, "sgala");
        assert_eq!(found[0].full.as_deref(), Some("segala"));
        assert_eq!(found[2].full.as_deref(), Some("karena"));
        assert_eq!(found[3].full, None);
        let text = "S'gala hormat";
        let ranges = highlight_ranges(text, &["segala".into()]);
        assert_eq!(&text[ranges[0].clone()], "S'gala");
    }

    #[test]
    fn highlight_joins_adjacent_words() {
        let text = "Morning light, morning light again";
        let ranges = highlight_ranges(text, &["morning".into(), "light".into()]);
        assert_eq!(&text[ranges[0].clone()], "Morning light");
        assert_eq!(ranges.len(), 2);
    }
}
