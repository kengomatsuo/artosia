//! Sorting the lines of text taken from a page: sung, section label, chords or noise, and
//! gathering the sung ones into stanzas.
//!
//! Chord lines follow ChordPro's two tests (A2Crd): spaces at least as many as the other
//! characters, or more than 40% of the words parse as chords.

use crate::text::tidy;

/// A line longer than this, on a page we do not know, is prose and never a sung line.
const LONGEST_LINE: usize = 90;
/// Fewer sung lines than this, on a page we do not know, is a caption or a menu.
pub(crate) const FEWEST_LINES: usize = 4;
/// More blank lines than this in a row end a song.
const MOST_BLANKS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Line {
    Blank,
    /// Chords, tablature, a repeat mark, a scripture reference: not sung, no break.
    Drop,
    /// Prose, a caption or a pointer to another page: the song cannot run through it.
    Break,
    /// A section label such as `Reff` or `Bait 2`.
    Label(String),
    Sung(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stanza {
    pub label: Option<String>,
    pub lines: Vec<String>,
}

/// Section words that mark sung parts.
const SUNG_LABELS: &[&str] = &[
    "verse", "ayat", "bait", "couplet", "bridge", "chorus", "reff", "reffrain", "refrain", "refrein", "ref", "kor", "koor", "tag", "hook",
];
/// Section words that mark parts nobody sings the words of.
const PLAIN_LABELS: &[&str] = &["intro", "interlude", "instrumental", "instrumentals", "outro", "coda", "ending", "solo", "musik", "int", "breakdown"];
const CHORUS_WORDS: &[&str] = &["chorus", "reff", "reffrain", "refrain", "refrein", "ref", "kor", "koor"];
/// Starts of lines that end a song on an unknown page.
const ASIDES: &[&str] = &[
    "foto:", "sumber:", "baca juga", "lihat juga", "gambar:", "image:", "source:", "iklan", "advertisement",
];
/// Starts of a credit or source line, dropped only at the end of the words.
const CREDITS: &[&str] = &[
    "©", "(c)", "copyright", "cipt", "pencipta", "lirik oleh", "lagu oleh", "lyrics by", "words and music", "words by", "music by",
    "written by", "composed", "sumber", "source", "ccli", "songwriter",
];
/// Starts of a line about how to play, not about what is sung.
const META: &[&str] = &["capo", "kunci dasar", "kunci:", "nada dasar", "tempo", "bpm", "tuning", "key:"];
/// Starts of a line that tells the band to repeat a part ("Back to Reff"): a direction, never sung.
const REPEATS: &[&str] = &["back to", "kembali ke", "balik ke", "ulangi", "ulang reff", "repeat", "(repeat", "go to", "ke reff"];

/// Sorts one line of text. `trusted` is a container a site's selector chose: prose and
/// captions there are dropped instead of ending the song.
pub(crate) fn classify(raw: &str, trusted: bool, out: &mut Vec<Line>) {
    let raw = raw.trim_end();
    let line = raw.trim();
    if line.is_empty() {
        out.push(Line::Blank);
        return;
    }
    let lower = line.to_lowercase();
    if let Some(label) = label_of(line) {
        match label.sung {
            true => out.push(Line::Label(label.text)),
            false => out.push(Line::Blank),
        }
        if let Some(rest) = label.rest {
            classify(&rest, trusted, out);
        }
        return;
    }
    let starts = |list: &[&str]| list.iter().any(|word| lower.starts_with(word));
    if is_underline(line)
        || is_tab(line)
        || is_chord_line(raw)
        || starts(META)
        || starts(REPEATS)
        || is_reference(line)
        || !line.chars().any(char::is_alphabetic)
    {
        out.push(Line::Drop);
    } else if starts(ASIDES) || line.contains("://") || line.contains('©') {
        out.push(if trusted { Line::Drop } else { Line::Break });
    } else if !trusted && line.chars().count() > LONGEST_LINE {
        out.push(Line::Break);
    } else {
        out.push(Line::Sung(line.split_whitespace().collect::<Vec<_>>().join(" ")));
    }
}

struct Label {
    text: String,
    sung: bool,
    /// What follows the label on its line, when that is words.
    rest: Option<String>,
}

/// `Reff`, `Chorus :`, `[Verse 1: Name]`, `Bait 2`, `Intro : G D Am C`, `Reff: words`.
fn label_of(line: &str) -> Option<Label> {
    let (inner, bracketed) = if line.starts_with(['[', '(']) {
        // `[Chorus]`, `((Reff)) 3x`: only marks may follow the closing bracket.
        let body = line.trim_start_matches(['[', '(']);
        let end = body.find([']', ')'])?;
        let tail = body[end..].trim_start_matches([']', ')']).trim();
        if !(tail.is_empty() || is_chords_or_marks(tail)) {
            return None;
        }
        (body[..end].trim(), true)
    } else {
        (line, false)
    };
    let word_end = inner.find(|c: char| !(c.is_alphabetic() || c == '-')).unwrap_or(inner.len());
    let (word, mut after) = inner.split_at(word_end);
    let key = word.to_lowercase().replace('-', "");
    // "Pre Chorus", "Pra-reff": a prefix word then a chorus word.
    let mut text = word.to_string();
    let mut key_final = key.clone();
    if matches!(key.as_str(), "pre" | "pra" | "post") {
        let next = after.trim_start();
        let end = next.find(|c: char| !c.is_alphabetic()).unwrap_or(next.len());
        if CHORUS_WORDS.contains(&next[..end].to_lowercase().as_str()) {
            text = format!("{word} {}", &next[..end]);
            key_final = "chorus".to_string();
            after = &next[end..];
        }
    } else if let Some(tail) = key.strip_prefix("pre").or_else(|| key.strip_prefix("pra")).or_else(|| key.strip_prefix("post"))
        && CHORUS_WORDS.contains(&tail)
    {
        key_final = "chorus".to_string();
    }
    let sung = SUNG_LABELS.contains(&key_final.as_str());
    if !sung && !PLAIN_LABELS.contains(&key_final.as_str()) {
        return None;
    }
    let after = after.trim_start();
    let digits = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
    if digits > 0 {
        text = format!("{text} {}", &after[..digits]);
    }
    let after = &after[digits..];
    let colon = after.trim_start().starts_with([':', '.']);
    let rest = after.trim_start().trim_start_matches([':', '.']).trim();
    let rest = if rest.is_empty() || bracketed || is_chords_or_marks(rest) {
        None
    } else if colon {
        Some(rest.to_string())
    } else {
        return None;
    };
    Some(Label { text, sung, rest })
}

/// Underline: a line of only `-`, `=`, `_` or `*`.
fn is_underline(line: &str) -> bool {
    line.chars().count() >= 3 && line.chars().all(|c| matches!(c, '-' | '=' | '_' | '*' | '~' | '.'))
}

/// Tablature: 80% of the characters are `.`, `-`, `|` or digits, or a string name opens it.
fn is_tab(line: &str) -> bool {
    let chars: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    let tabby = chars.iter().filter(|c| matches!(c, '.' | '-' | '|') || c.is_ascii_digit()).count();
    let dashes = chars.iter().any(|c| matches!(c, '-' | '|'));
    let string_name = matches!(chars.as_slice(), [s, p, ..] if "eEBGDAa".contains(*s) && matches!(p, '|' | ':'));
    dashes && ((chars.len() >= 6 && tabby * 5 >= chars.len() * 4) || (string_name && line.contains('-')))
}

/// `[ Mzm 104:1-7 ]`: a scripture reference a hymn page puts between verses.
fn is_reference(line: &str) -> bool {
    let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else { return false };
    let inner = inner.trim();
    inner.chars().next().is_some_and(|c| c.is_alphanumeric()) && inner.chars().any(|c| c.is_ascii_digit()) && inner.chars().any(char::is_alphabetic)
}

/// A chord line by ChordPro's two tests, or a line of only marks (`N.C.`, `(2x)`, `x2`).
fn is_chord_line(raw: &str) -> bool {
    let spaces = raw.chars().filter(|c| matches!(c, ' ' | '\t')).count();
    let others = raw.chars().filter(|c| !c.is_whitespace()).count();
    spaces >= others || is_chords_or_marks(raw) || chord_share(raw)
}

/// More than 40% of the words that are not marks are chords. One chord among words is not
/// enough ("A song", "Am I"): the line needs two, or to be that one word.
fn chord_share(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().filter(|w| !is_mark(w)).collect();
    let chords = words.iter().filter(|w| is_chord(w)).count();
    (chords >= 2 || words.len() == 1) && chords > 0 && chords * 5 > words.len() * 2
}

/// Every word is a chord or a mark, and at least one word is there.
fn is_chords_or_marks(text: &str) -> bool {
    let mut words = text.split_whitespace().peekable();
    words.peek().is_some() && words.all(|w| is_mark(w) || is_chord(w))
}

/// Marks that sit in a chord line: bars, `N.C.`, repeat counts.
fn is_mark(word: &str) -> bool {
    let core = word.trim_matches(['(', ')', ':']);
    if core.is_empty() || core.chars().all(|c| matches!(c, '|' | '/' | '-' | '.' | '%' | '*' | '~')) {
        return true;
    }
    if matches!(core.to_ascii_lowercase().as_str(), "n.c." | "n.c" | "nc" | "x") {
        return true;
    }
    let repeat = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    match core.strip_prefix(['x', 'X']).or_else(|| core.strip_suffix(['x', 'X'])) {
        Some(count) => repeat(count),
        None => false,
    }
}

/// A chord: root, quality, extensions (`7`, `sus4`, `add9`, `b5`, `#9`), optional bass.
pub(crate) fn is_chord(word: &str) -> bool {
    let word = word.trim_matches(['|', ':', ',']);
    let (main, bass) = match word.split_once('/') {
        Some((main, bass)) => (main, Some(bass)),
        None => (word, None),
    };
    let Some(mut rest) = strip_note(main) else { return false };
    for quality in ["maj", "min", "dim", "aug", "m", "M", "+"] {
        if let Some(after) = rest.strip_prefix(quality) {
            rest = after;
            break;
        }
    }
    while !rest.is_empty() {
        let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest = if digits > 0 {
            &rest[digits..]
        } else if let Some(after) = ["sus", "add", "maj", "dim", "aug", "no"].iter().find_map(|w| rest.strip_prefix(w)) {
            after
        } else if let Some(after) = rest.strip_prefix(['b', '#']).filter(|a| a.starts_with(|c: char| c.is_ascii_digit())) {
            after
        } else if let Some(after) = rest.strip_prefix(['(', ')', ',', '+', '-']) {
            after
        } else {
            return false;
        };
    }
    bass.is_none_or(|bass| strip_note(bass).is_some_and(str::is_empty))
}

/// The text after a note name `A` to `G` with an optional `#` or `b`.
fn strip_note(part: &str) -> Option<&str> {
    let rest = part.strip_prefix(['A', 'B', 'C', 'D', 'E', 'F', 'G'])?;
    Some(rest.strip_prefix(['#', 'b']).unwrap_or(rest))
}

/// The best run of lines on an unknown page: the most sung lines with no break through them.
pub(crate) fn best_run(text: &str) -> Option<Vec<Stanza>> {
    let mut lines = Vec::new();
    text.lines().for_each(|raw| classify(raw, false, &mut lines));
    let mut best: Option<(usize, &[Line])> = None;
    let mut start = 0;
    let mut sung = 0;
    let mut blanks = 0;
    for at in 0..=lines.len() {
        let line = lines.get(at);
        blanks = if line == Some(&Line::Blank) { blanks + 1 } else { 0 };
        if line.is_some() && line != Some(&Line::Break) && blanks <= MOST_BLANKS {
            sung += usize::from(matches!(line, Some(Line::Sung(_))));
            continue;
        }
        if best.is_none_or(|(most, _)| sung > most) {
            best = Some((sung, &lines[start..at]));
        }
        start = at + 1;
        sung = 0;
        blanks = 0;
    }
    let (sung, run) = best?;
    (sung >= FEWEST_LINES).then(|| finish(stanzas(run)))
}

/// The words of a trusted container or a structured field.
pub(crate) fn trusted(text: &str) -> Vec<Stanza> {
    let mut lines = Vec::new();
    text.lines().for_each(|raw| classify(raw, true, &mut lines));
    finish(stanzas(&lines))
}

pub(crate) fn sung_count(stanzas: &[Stanza]) -> usize {
    stanzas.iter().map(|s| s.lines.len()).sum()
}

fn stanzas(lines: &[Line]) -> Vec<Stanza> {
    let mut out: Vec<Stanza> = Vec::new();
    let mut open = false;
    for line in lines {
        match line {
            Line::Blank => open = out.last().is_some_and(|s| s.lines.is_empty() && s.label.is_some()),
            Line::Label(text) => {
                match out.last_mut() {
                    Some(last) if last.lines.is_empty() => last.label = Some(text.clone()),
                    _ => out.push(Stanza { label: Some(text.clone()), lines: Vec::new() }),
                }
                open = true;
            }
            Line::Sung(text) => {
                if !open || out.is_empty() {
                    out.push(Stanza { label: None, lines: Vec::new() });
                }
                open = true;
                if let Some(last) = out.last_mut() {
                    last.lines.push(text.clone());
                }
            }
            Line::Drop | Line::Break => {}
        }
    }
    out
}

/// Drops hymn verse numbers and trailing credits, each only when a check proves it.
fn finish(mut stanzas: Vec<Stanza>) -> Vec<Stanza> {
    strip_verse_numbers(&mut stanzas);
    while let Some(last) = stanzas.last_mut() {
        if last.lines.last().is_some_and(|line| is_credit(line)) {
            last.lines.pop();
        }
        if last.lines.is_empty() {
            stanzas.pop();
        } else {
            break;
        }
    }
    stanzas.retain(|s| !s.lines.is_empty());
    stanzas
}

fn is_credit(line: &str) -> bool {
    let lower = line.to_lowercase();
    CREDITS.iter().any(|word| lower.starts_with(word))
}

/// `3. words` to `words` in every stanza that has a number, when those numbers run 1, 2, 3...
/// in order (a lone `1` needs a `.` or `)` after it).
fn strip_verse_numbers(stanzas: &mut [Stanza]) {
    let number = |line: &str| -> Option<(u32, bool, usize)> {
        let digits = line.find(|c: char| !c.is_ascii_digit())?;
        let (n, rest) = line.split_at(digits);
        let marked = rest.starts_with(['.', ')']);
        let after = rest.trim_start_matches(['.', ')']).trim_start();
        let gap = line.len() - after.len();
        (digits > 0 && digits <= 2 && (marked || rest.starts_with(' ')) && after.chars().any(char::is_alphabetic))
            .then(|| (n.parse().unwrap_or(0), marked, gap))
    };
    let found: Vec<(usize, u32, bool, usize)> = stanzas
        .iter()
        .enumerate()
        .filter_map(|(at, s)| s.lines.first().and_then(|l| number(l)).map(|(n, marked, gap)| (at, n, marked, gap)))
        .collect();
    let counted = found.iter().enumerate().all(|(i, (_, n, _, _))| *n as usize == i + 1);
    let proven = !found.is_empty() && counted && (found.len() > 1 || found[0].2);
    if proven {
        for (at, _, _, gap) in found {
            stanzas[at].lines[0] = stanzas[at].lines[0][gap..].to_string();
        }
    }
}

/// Stanzas as blank-line separated text; a label is a `[Reff]` line the song reader knows.
pub(crate) fn render(stanzas: &[Stanza]) -> String {
    let parts: Vec<String> = stanzas
        .iter()
        .map(|s| match &s.label {
            Some(label) => format!("[{label}]\n{}", s.lines.join("\n")),
            None => s.lines.join("\n"),
        })
        .collect();
    tidy(&parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str, trusted: bool) -> Vec<Line> {
        let mut out = Vec::new();
        text.lines().for_each(|l| classify(l, trusted, &mut out));
        out
    }

    #[test]
    fn a_repeat_direction_is_dropped_and_never_sung() {
        for line in ["Back to Reff", "Back to *, Reff", "Kembali ke Reff", "Ulangi reff 2x", "(Repeat chorus)"] {
            assert_eq!(kinds(line, true), vec![Line::Drop], "{line}");
        }
        // A sung line that merely holds the words stays sung.
        assert!(matches!(kinds("Bawa aku kembali ke rumah-Mu", true)[..], [Line::Sung(_)]));
    }

    #[test]
    fn real_chord_shapes_parse_and_words_do_not() {
        for chord in ["C", "C7sus4", "Cm7b5", "Gadd9", "D/F#", "Bbmaj7", "F#m7", "Asus4", "Em", "G/B", "C#dim", "Abm", "Cmaj7", "E7#9", "D(add9)"] {
            assert!(is_chord(chord), "{chord}");
        }
        for word in ["Dia", "Aku", "Bad", "Add", "Cab", "God", "Be", "love", "am", "Gloria"] {
            assert!(!is_chord(word), "{word}");
        }
    }

    #[test]
    fn chord_lines_fall_by_space_ratio_or_chord_share() {
        assert!(is_chord_line("            C        Cmaj7"));
        assert!(is_chord_line("C Dm Em F G Am Bdim"));
        assert!(is_chord_line("G  D  tinggi"), "two of three words are chords");
        assert!(!is_chord_line("A Friend Goes Before"));
        assert!(!is_chord_line("Dia baik"));
        assert!(is_chord_line("N.C.") && is_chord_line("(2x)") && is_chord_line("x2") && is_chord_line("| C | G |"));
    }

    #[test]
    fn labels_noise_and_sung_lines_are_told_apart() {
        let lines = kinds("Intro : G D Am C\nReff:\n[Verse 1: Someone]\nBait 2\n-----\ne|--3--0--|\nCapo 2nd fret\n[ Mzm 104:1-7 ]\nBait pertama kali\nSumber: situs\nIntro", false);
        assert_eq!(
            lines,
            [
                Line::Blank,
                Line::Label("Reff".into()),
                Line::Label("Verse 1".into()),
                Line::Label("Bait 2".into()),
                Line::Drop,
                Line::Drop,
                Line::Drop,
                Line::Drop,
                Line::Sung("Bait pertama kali".into()),
                Line::Break,
                Line::Blank,
            ]
        );
        assert_eq!(kinds("((Chorus)) 3x
[Post-Chorus]
[Instrumentals]
(kata kata)", true), [Line::Label("Chorus".into()), Line::Label("Post-Chorus".into()), Line::Blank, Line::Sung("(kata kata)".into())]);
        assert_eq!(kinds("Reff: kata kata", true), [Line::Label("Reff".into()), Line::Sung("kata kata".into())]);
    }

    #[test]
    fn verse_numbers_go_only_when_they_run_in_order() {
        let mut lines = Vec::new();
        for text in ["1. Satu dua\ntiga empat", "", "Reff", "Lima enam", "", "2. Tujuh delapan", "", "3 Sembilan sepuluh"] {
            text.lines().for_each(|l| classify(l, true, &mut lines));
            if text.is_empty() {
                lines.push(Line::Blank);
            }
        }
        let done = finish(stanzas(&lines));
        assert_eq!(done[0].lines[0], "Satu dua");
        assert_eq!(done[2].lines[0], "Tujuh delapan");
        assert_eq!(done[3].lines[0], "Sembilan sepuluh");
        let mut odd = stanzas(&kinds("2 menjadi satu\n\n5 lagi disini", true));
        strip_verse_numbers(&mut odd);
        assert_eq!(odd[0].lines[0], "2 menjadi satu");
        let mut lone = stanzas(&kinds("1 Korintus kata\nbaris dua", true));
        strip_verse_numbers(&mut lone);
        assert_eq!(lone[0].lines[0], "1 Korintus kata");
    }

    #[test]
    fn a_trailing_credit_goes_and_a_middle_one_stays() {
        let kept = trusted("Baris satu\nBaris dua\n\nCipt. Seseorang\nBaris tiga\n\n(c) Contoh Musik");
        assert_eq!(render(&kept), "Baris satu\nBaris dua\n\nCipt. Seseorang\nBaris tiga");
    }

    #[test]
    fn the_best_run_skips_menus_prose_and_chords() {
        let prose = "Artikel ini panjang sekali dan menjelaskan latar belakang lagu ini dengan banyak kalimat yang tidak dinyanyikan.";
        let text = format!("Beranda\nMasuk\n{prose}\nBaris satu\nG D\nBaris dua\n\nReff\nBaris tiga\nBaris empat\n{prose}\nSatu\nDua");
        let run = best_run(&text).unwrap();
        assert_eq!(render(&run), "Baris satu\nBaris dua\n\n[Reff]\nBaris tiga\nBaris empat");
        assert_eq!(best_run("Satu\nDua\nTiga"), None);
    }
}
