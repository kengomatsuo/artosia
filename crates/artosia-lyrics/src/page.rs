//! A song's words from a web page the person chose: a Gema SABDA song page, or any lyrics page
//! whose address they pasted. The person presses Import; Artosia reads that one page.
//!
//! Pages carry no common markup for lyrics, so the words are found by shape: the longest run of
//! short lines, stanzas allowed between them, that is not menu or footer text.

use crate::text::tidy;
use crate::{Fetched, LyricsError, http};

/// A line longer than this is prose (an article, a note), never a sung line.
const LONGEST_LINE: usize = 90;
/// Fewer lines than this is a caption or a menu, not a song.
const FEWEST_LINES: usize = 4;

/// Reads the words of the page at `url`.
pub(crate) fn from_page(url: &str) -> Result<Fetched, LyricsError> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(LyricsError::BadAnswer("the address does not start with https://".to_string()));
    }
    let html = http::get(url, &[])?;
    words_of(&html, url)
}

/// The words of a page already fetched; `url` names the source.
pub(crate) fn words_of(html: &str, url: &str) -> Result<Fetched, LyricsError> {
    let site = site_of(url);
    let title = page_title(html, &site);
    // A Gema page keeps its words in the body field; read only that, so its menus never count.
    let scope = gema_body(html).unwrap_or(html);
    let text = lyric_block(&to_text(scope)).ok_or(LyricsError::NotFound)?;
    Ok(Fetched { title, artist: String::new(), text, source: site, url: url.to_string() })
}

/// The host without `www.`: the credit line names the site the words came from.
pub(crate) fn site_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.trim_start_matches("www.").to_string()
}

/// The page's title with the site name and "lirik" words taken off.
fn page_title(html: &str, site: &str) -> String {
    let raw = between(html, "<title", "</title>").map(|t| t.split_once('>').map_or(t, |(_, v)| v)).unwrap_or_default();
    let mut title = decode(raw.trim());
    // "KJ 004. Hai Mari Sembah | GEMA", "Lirik Lagu X - Situs": the first part is the song.
    for cut in [" | ", " - ", " – ", " — "] {
        if let Some((head, _)) = title.split_once(cut) {
            title = head.to_string();
        }
    }
    let lower = title.to_lowercase();
    for prefix in ["lirik lagu rohani ", "lirik lagu ", "lirik "] {
        if lower.starts_with(prefix) {
            title = title[prefix.len()..].to_string();
            break;
        }
    }
    let title = title.trim().to_string();
    if title.is_empty() { site.to_string() } else { title }
}

fn gema_body(html: &str) -> Option<&str> {
    let at = html.find("field--name-body")?;
    // Start after the opening tag, so its attributes are never read as words.
    let rest = &html[at..];
    let rest = &rest[rest.find('>').map_or(0, |gt| gt + 1)..];
    let end = rest.find("field--name-field").or_else(|| rest.find("<footer")).unwrap_or(rest.len());
    Some(&rest[..end])
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(&text[start..end])
}

/// HTML to plain text: a line break per `<br>`, a blank line per closing block, scripts,
/// styles and comments dropped, entities decoded.
pub(crate) fn to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map_or("", |end| &rest[end + 3..]);
            continue;
        }
        // A tag cut off at the end of the text is dropped, never read as words.
        let Some(gt) = rest.find('>') else {
            rest = "";
            break;
        };
        let tag = rest[1..gt].trim().to_ascii_lowercase();
        rest = &rest[gt + 1..];
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        if !tag.starts_with('/') && matches!(name, "script" | "style" | "noscript" | "svg" | "nav" | "header" | "footer" | "aside" | "form") {
            let close = format!("</{name}");
            rest = rest.to_ascii_lowercase().find(&close).map_or("", |end| {
                let after = &rest[end..];
                after.find('>').map_or("", |gt| &after[gt + 1..])
            });
            out.push_str("\n\n");
            continue;
        }
        match name {
            "br" => out.push('\n'),
            "p" | "div" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "tr" | "table" | "ul" | "ol" | "section" | "article" | "blockquote" | "pre" => {
                out.push_str("\n\n")
            }
            _ => {}
        }
    }
    out.push_str(rest);
    decode(&out)
}

/// The longest run of short lines, stanzas allowed, as tidy blank-line separated text.
pub(crate) fn lyric_block(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut best: Option<(usize, usize, usize)> = None; // (sung lines, start, end)
    let mut start = 0;
    while start < lines.len() {
        if !is_sung(lines[start]) {
            start += 1;
            continue;
        }
        let mut end = start;
        let mut sung = 0;
        let mut blank_run = 0;
        let mut at = start;
        while at < lines.len() {
            let line = lines[at];
            if line.is_empty() || is_chords(line) {
                // Chord lines sit between sung lines on guitar pages: passed over, never kept.
                blank_run += usize::from(line.is_empty());
                // Stanzas are a blank line or two apart; more ends the song.
                if blank_run > 4 {
                    break;
                }
            } else if is_sung(line) {
                blank_run = 0;
                sung += 1;
                end = at;
            } else {
                break;
            }
            at += 1;
        }
        if best.is_none_or(|(most, _, _)| sung > most) {
            best = Some((sung, start, end));
        }
        start = at.max(start + 1);
    }
    let (sung, start, end) = best?;
    let kept: Vec<&str> = lines[start..=end].iter().copied().filter(|line| !is_chords(line)).collect();
    (sung >= FEWEST_LINES).then(|| tidy(&kept.join("\n")))
}

fn is_sung(line: &str) -> bool {
    let chars = line.chars().count();
    let lower = line.to_lowercase();
    // A photo caption or a pointer to another article ends the song.
    let aside = ["foto:", "sumber:", "baca juga", "gambar:", "image:", "source:", "iklan", "advertisement"]
        .iter()
        .any(|word| lower.starts_with(word));
    chars > 0
        && chars <= LONGEST_LINE
        && !aside
        && line.chars().any(char::is_alphabetic)
        && !line.contains("://")
        && !line.contains('©')
        && !is_chords(line)
}

/// A line of guitar chords only: every word is a chord such as `D`, `Bm`, `F#m7` or `G/B`.
fn is_chords(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    !words.is_empty() && words.iter().all(|word| is_chord(word.trim_matches(|c| c == '|' || c == '-')))
}

fn is_chord(word: &str) -> bool {
    if word.is_empty() {
        return true;
    }
    let (main, bass) = word.split_once('/').unwrap_or((word, ""));
    let note = |part: &str| -> Option<usize> {
        let mut chars = part.chars();
        let root = chars.next()?;
        if !('A'..='G').contains(&root) {
            return None;
        }
        Some(1 + usize::from(matches!(chars.next(), Some('#' | 'b'))))
    };
    let Some(root) = note(main) else { return false };
    let mut rest = &main[root.min(main.len())..];
    for quality in ["maj", "min", "dim", "aug", "m"] {
        if let Some(after) = rest.strip_prefix(quality) {
            rest = after;
            break;
        }
    }
    for extra in ["sus", "add"] {
        if let Some(after) = rest.strip_prefix(extra) {
            rest = after;
        }
    }
    rest.chars().all(|c| c.is_ascii_digit()) && (bass.is_empty() || note(bass).is_some_and(|n| n == bass.len()))
}

/// Decodes the entities pages use: named ones for markup and spaces, and numeric ones.
pub(crate) fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..semi];
        let ch = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            "rsquo" | "lsquo" => Some('\''),
            "rdquo" | "ldquo" => Some('"'),
            "ndash" | "mdash" => Some('-'),
            _ if name.starts_with("#x") || name.starts_with("#X") => u32::from_str_radix(&name[2..], 16).ok().and_then(char::from_u32),
            _ if name.starts_with('#') => name[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(ch) => {
                out.push(ch);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEMA: &str = r#"<html><head><title>KJ 004. Hai Mari Sembah | GEMA</title></head><body>
<nav><a href="/">Beranda</a><a href="/daftar_lirik">Daftar Lirik</a><a>Kidung</a><a>Kontak</a></nav>
<div class="clearfix text-formatted field field--name-body field--type-text-with-summary"><p>Baris satu contoh<br>Baris dua contoh<br>Baris tiga contoh<br>Baris empat contoh</p>
<p>Bait dua baris satu<br>Bait dua baris dua<br>Bait dua baris tiga<br>Bait dua baris empat</p></div>
<div class="field field--name-field-kategori">Kategori: Pujian</div>
<footer>Hak cipta &copy; 2006 YLSA. All Rights Reserved.</footer></body></html>"#;

    #[test]
    fn a_gema_page_gives_its_title_and_stanzas() {
        let song = words_of(GEMA, "https://gema.sabda.org/kj_004_hai_mari_sembah").unwrap();
        assert_eq!(song.title, "KJ 004. Hai Mari Sembah");
        assert_eq!(song.source, "gema.sabda.org");
        assert_eq!(song.text.split("\n\n").count(), 2);
        assert_eq!(song.text.lines().filter(|l| !l.is_empty()).count(), 8);
        assert!(!song.text.contains("Beranda") && !song.text.contains("Kategori"));
    }

    #[test]
    fn a_blog_page_gives_the_song_and_not_the_article() {
        let html = format!(
            "<title>Lirik Lagu Contoh Pujian - Blog Contoh</title><p>{}</p><div>Contoh baris satu<br>Contoh baris dua<br>Contoh baris tiga</div><div>Reff:<br>Contoh reff satu<br>Contoh reff dua</div><p>{}</p>",
            "Artikel ini panjang sekali dan menjelaskan latar belakang lagu ini dengan banyak kalimat yang tidak dinyanyikan sama sekali.",
            "Baca juga: artikel lain yang sangat panjang dengan banyak kalimat dan tautan ke halaman lain di situs ini."
        );
        let song = words_of(&html, "https://www.contoh.example/lirik").unwrap();
        assert_eq!(song.title, "Contoh Pujian");
        assert_eq!(song.source, "contoh.example");
        assert_eq!(song.text, "Contoh baris satu\nContoh baris dua\nContoh baris tiga\n\nReff:\nContoh reff satu\nContoh reff dua");
    }

    #[test]
    fn a_page_with_no_song_is_not_found() {
        let html = "<title>Beranda</title><p>Satu kalimat panjang yang tidak berbentuk lagu sama sekali karena hanya prosa biasa saja di sini.</p>";
        assert_eq!(words_of(html, "https://contoh.example/").unwrap_err(), LyricsError::NotFound);
    }

    #[test]
    fn chord_lines_are_dropped_and_a_caption_ends_the_song() {
        let html = "<p>D A Bm A</p><p>Contoh baris satu<br>G/B D<br>Contoh baris dua<br>Contoh baris tiga<br>F#m7 Gmaj7 Asus4<br>Contoh baris empat</p><p>Foto: Ilustrasi contoh (contoh.example)</p>";
        let song = words_of(html, "https://contoh.example/a").unwrap();
        assert_eq!(song.text, "Contoh baris satu\nContoh baris dua\nContoh baris tiga\nContoh baris empat");
        assert!(is_chords("C Dm Em F G Am Bdim") && !is_chords("Dia baik") && !is_chords("Aku"));
    }

    #[test]
    fn entities_and_addresses_are_read() {
        assert_eq!(decode("&#039;Ku Tak Dapat &amp; &#x41;&nbsp;b &bogus x"), "'Ku Tak Dapat & A b &bogus x");
        assert_eq!(site_of("https://www.example.org/a?b"), "example.org");
        assert!(from_page("gema.sabda.org/kj").is_err());
    }
}
