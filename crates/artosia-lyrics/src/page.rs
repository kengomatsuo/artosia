//! A song's words from a web page the person chose: a Gema SABDA song page, or any lyrics page
//! whose address they pasted. The person presses Import; Artosia reads that one page.
//!
//! Three routes, in order: a per-site selector table, the page's JSON-LD `lyrics.text`, then a
//! shape finder run on each candidate container. A `<br>` and every block end become a newline
//! before anything is measured, and ad and header nodes are removed by node, not by line.

use scraper::{ElementRef, Html, Node, Selector};

use crate::lines::{self, Stanza};
use crate::{Fetched, LyricsError, http};

/// Fewer sung lines than this from a site's own selector or JSON-LD sends the reader on.
const FEWEST_TRUSTED: usize = 2;
/// A container with at least this share of its text (1 in N) inside links is a menu.
const LINK_SHARE: usize = 2;

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
    let doc = Html::parse_document(html);
    let (title, artist) = page_title(&doc, &site);
    let stanzas = by_site(&doc, &site).or_else(|| by_json_ld(&doc)).or_else(|| by_shape(&doc)).ok_or(LyricsError::NotFound)?;
    Ok(Fetched { title, artist, text: lines::render(&stanzas), source: site, url: url.to_string() })
}

/// The host without `www.`: the credit line names the site the words came from.
pub(crate) fn site_of(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.trim_start_matches("www.").to_string()
}

/// The song's title and artist: the site's own rule first, then the cut `<title>` with no artist.
fn page_title(doc: &Html, site: &str) -> (String, String) {
    site_title(doc, site).filter(|(title, _)| !title.is_empty()).unwrap_or_else(|| (cut_title(doc, site), String::new()))
}

/// A structured source per site: `og:title`, or the lyric header element on AZLyrics.
fn site_title(doc: &Html, site: &str) -> Option<(String, String)> {
    let on = |host: &str| site == host || site.ends_with(&format!(".{host}"));
    let og = select(doc.root_element(), r#"meta[property="og:title"]"#).next().and_then(|m| m.attr("content")).map(clean);
    let (title, artist) = if on("chordtela.com") {
        // "Kunci Gitar Artist - Song Chord Dasar ©ChordTela.com"
        let head = og.as_deref()?.split('©').next()?;
        let (artist, song) = head.split_once(" - ")?;
        let song = song.rfind(" Chord").map_or(song, |at| &song[..at]);
        (song.to_string(), artist.strip_prefix("Kunci Gitar ").unwrap_or(artist).to_string())
    } else if on("genius.com") {
        // "Artist (Ft. Other) – Song"
        let (artist, song) = og.as_deref()?.split_once(" – ")?;
        (song.to_string(), artist.to_string())
    } else if on("lirik.kapanlagi.com") {
        // "Lirik Song - Artist | Makna dan Arti Lagu", "Terjemahan Song - Artist | Arti Lirik"
        let head = og.as_deref()?.split(" | ").next()?;
        let (song, artist) = head.rsplit_once(" - ")?;
        let lower = song.to_lowercase();
        let skip = ["lirik ", "terjemahan "].iter().find(|p| lower.starts_with(*p)).map_or(0, |p| p.len());
        (song[skip..].to_string(), artist.to_string())
    } else if on("azlyrics.com") {
        // The header reads `"Song" lyrics`.
        let h1 = select(doc.root_element(), "h1").next().map(|h| clean(&h.text().collect::<String>()))?;
        let song = h1.strip_suffix(" lyrics").unwrap_or(&h1).trim_matches('"').to_string();
        // The <title> reads `Artist - Song Lyrics | AZLyrics.com`.
        let named: String = select(doc.root_element(), "title").next().map(|t| clean(&t.text().collect::<String>())).unwrap_or_default();
        let artist = named.split_once(" - ").map_or("", |(artist, _)| artist);
        (song, artist.to_string())
    } else {
        return None;
    };
    Some((title.trim().to_string(), artist.trim().to_string()))
}

/// One space between words; a no-break space is a space.
fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The page's title with the site name and "lirik" words taken off.
fn cut_title(doc: &Html, site: &str) -> String {
    let raw: String = select(doc.root_element(), "title").next().map(|t| t.text().collect()).unwrap_or_default();
    let mut title = raw.trim().to_string();
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

fn select<'a>(root: ElementRef<'a>, css: &str) -> impl Iterator<Item = ElementRef<'a>> + use<'a> {
    let found: Vec<ElementRef<'a>> = Selector::parse(css).map(|s| root.select(&s).collect()).unwrap_or_default();
    found.into_iter()
}

// ---- route 1: the site table ------------------------------------------------------------

enum Take {
    First,
    All,
    /// Each match is a stanza.
    Paragraphs,
    Longest,
}

struct Site {
    host: &'static str,
    pick: &'static str,
    take: Take,
    /// Nodes cut out of the container before its text is read.
    drop: &'static [&'static str],
    /// Text from this marker on is a second copy or a footer.
    stop: Option<&'static str>,
}

const SITES: &[Site] = &[
    Site { host: "gema.sabda.org", pick: "div.field--name-body", take: Take::First, drop: &[], stop: None },
    Site { host: "lirik.kapanlagi.com", pick: "div.lyric-body", take: Take::First, drop: &[], stop: None },
    Site { host: "chordtela.com", pick: "div.post-body pre", take: Take::First, drop: &["span.custom"], stop: Some("===ORIGINAL CHORD===") },
    Site {
        host: "genius.com",
        pick: "[data-lyrics-container=true]",
        take: Take::All,
        drop: &["[class*=LyricsHeader]", "[data-exclude-from-selection=true]"],
        stop: None,
    },
    Site { host: "letras.mus.br", pick: ".lyric-original p", take: Take::Paragraphs, drop: &[], stop: None },
    Site {
        host: "azlyrics.com",
        pick: ".col-xs-12.col-lg-8.text-center > div:not([class]):not([id])",
        take: Take::Longest,
        drop: &[],
        stop: None,
    },
];

fn by_site(doc: &Html, site: &str) -> Option<Vec<Stanza>> {
    let entry = SITES.iter().find(|s| site == s.host || site.ends_with(&format!(".{}", s.host)))?;
    let drop: Vec<Selector> = entry.drop.iter().filter_map(|css| Selector::parse(css).ok()).collect();
    let texts = select(doc.root_element(), entry.pick).map(|el| Walk::run(el, &drop, false).text);
    let text = match entry.take {
        Take::First => texts.take(1).collect::<String>(),
        Take::All => texts.collect::<Vec<_>>().join("\n"),
        Take::Paragraphs => texts.collect::<Vec<_>>().join("

"),
        Take::Longest => texts.max_by_key(String::len).unwrap_or_default(),
    };
    let text = entry.stop.and_then(|marker| text.split_once(marker)).map_or(text.as_str(), |(before, _)| before);
    let stanzas = lines::trusted(text);
    (lines::sung_count(&stanzas) >= FEWEST_TRUSTED).then_some(stanzas)
}

// ---- route 2: JSON-LD ---------------------------------------------------------------------

fn by_json_ld(doc: &Html) -> Option<Vec<Stanza>> {
    let text = select(doc.root_element(), r#"script[type="application/ld+json"]"#).find_map(|script| {
        let value: serde_json::Value = serde_json::from_str(&script.text().collect::<String>()).ok()?;
        lyrics_in(&value)
    })?;
    let stanzas = lines::trusted(&text);
    (lines::sung_count(&stanzas) >= FEWEST_TRUSTED).then_some(stanzas)
}

/// The `lyrics` property of a schema.org item: a string, or a CreativeWork with `text`.
fn lyrics_in(value: &serde_json::Value) -> Option<String> {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let own = map.get("lyrics").and_then(|l| match l {
                Value::String(text) => Some(text.clone()),
                Value::Object(work) => work.get("text").and_then(Value::as_str).map(str::to_string),
                _ => None,
            });
            own.or_else(|| map.values().find_map(lyrics_in))
        }
        Value::Array(items) => items.iter().find_map(lyrics_in),
        _ => None,
    }
}

// ---- route 3: the shape finder --------------------------------------------------------------

/// Class and id words Readability reads as page furniture; a positive word overrules them.
const NEGATIVE: &[&str] = &[
    "comment", "footer", "header", "menu", "related", "sidebar", "social", "sponsor", "share", "promo", "advert", "banner",
    "breadcrumb", "popup", "cookie", "newsletter", "widget", "recommend", "trending", "modal", "pagination", " ad ", " ads ",
];
const POSITIVE: &[&str] = &["article", "body", "content", "entry", "main", "post", "text", "story", "lyric", "lirik", "lagu", "song", "teks"];

/// Class and id of an element, lowercase, in spaces so a short word matches whole.
fn mark(el: ElementRef) -> String {
    format!(" {} {} ", el.value().attr("class").unwrap_or("").replace(['-', '_'], " "), el.value().id().unwrap_or("")).to_lowercase()
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// Readability's unlikely candidate: a negative word and no positive one.
fn unlikely(el: ElementRef) -> bool {
    let mark = mark(el);
    has_any(&mark, NEGATIVE) && !has_any(&mark, POSITIVE)
}

fn by_shape(doc: &Html) -> Option<Vec<Stanza>> {
    struct Candidate {
        sung: usize,
        class: i8,
        length: usize,
        stanzas: Vec<Stanza>,
    }
    let mut found: Vec<Candidate> = Vec::new();
    for el in select(doc.root_element(), "body, div, article, section, main, pre, td, blockquote, p, ul, ol, dd, center") {
        let walked = Walk::run(el, &[], true);
        if walked.total < 40 || walked.link * LINK_SHARE > walked.total {
            continue;
        }
        if let Some(stanzas) = lines::best_run(&walked.text) {
            let mark = mark(el);
            let class = i8::from(has_any(&mark, POSITIVE)) - i8::from(has_any(&mark, NEGATIVE));
            found.push(Candidate { sung: lines::sung_count(&stanzas), class, length: walked.text.len(), stanzas });
        }
    }
    // An ancestor holds the same run plus its neighbours: among the near-best, prefer the one
    // whose class names read as content, then the smallest.
    let most = found.iter().map(|c| c.sung).max()?;
    found
        .into_iter()
        .filter(|c| c.sung * 5 >= most * 4)
        .max_by_key(|c| (c.class, std::cmp::Reverse(c.length)))
        .map(|c| c.stanzas)
}

// ---- HTML to text ---------------------------------------------------------------------------

/// Tags whose content is never words.
const SKIPPED: &[&str] = &[
    "script", "style", "noscript", "svg", "template", "iframe", "button", "select", "textarea", "head", "nav", "form", "canvas", "audio", "video",
];
/// In loose mode these go too: page furniture.
const FURNITURE: &[&str] = &["header", "footer", "aside"];

/// Text of one container, with line breaks kept and nodes dropped by node.
struct Walk<'a> {
    drop: &'a [Selector],
    loose: bool,
    text: String,
    /// Characters of text, and how many of them sit inside links.
    total: usize,
    link: usize,
}

impl<'a> Walk<'a> {
    fn run(root: ElementRef, drop: &'a [Selector], loose: bool) -> Self {
        let mut walk = Walk { drop, loose, text: String::new(), total: 0, link: 0 };
        walk.children(root, root.value().name() == "pre", false);
        walk
    }

    fn children(&mut self, el: ElementRef, pre: bool, link: bool) {
        for child in el.children() {
            match child.value() {
                Node::Text(text) => self.words(text, pre, link),
                Node::Element(_) => {
                    if let Some(el) = ElementRef::wrap(child) {
                        self.element(el, pre, link);
                    }
                }
                _ => {}
            }
        }
    }

    fn element(&mut self, el: ElementRef, pre: bool, link: bool) {
        let name = el.value().name();
        let hidden = el.value().attr("hidden").is_some() || el.value().attr("style").is_some_and(|s| s.replace(' ', "").contains("display:none"));
        if hidden
            || SKIPPED.contains(&name)
            || self.drop.iter().any(|s| s.matches(&el))
            || (self.loose && (FURNITURE.contains(&name) || unlikely(el)))
        {
            return;
        }
        if name == "br" {
            self.text.push('\n');
            return;
        }
        let gap = match name {
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "ul" | "ol" | "table" | "pre" => 2,
            "div" | "li" | "tr" | "section" | "article" | "main" | "dd" | "dt" | "center" | "figure" | "details" => 1,
            _ => 0,
        };
        self.gap(gap);
        self.children(el, pre || name == "pre", link || name == "a");
        self.gap(gap);
    }

    /// Makes the text end with `lines` newlines, unless nothing is written yet.
    fn gap(&mut self, lines: usize) {
        if self.text.is_empty() {
            return;
        }
        let have = self.text.chars().rev().take_while(|c| *c == '\n').count();
        (have..lines).for_each(|_| self.text.push('\n'));
    }

    fn words(&mut self, text: &str, pre: bool, link: bool) {
        let kept = text.chars().filter(|c| !c.is_whitespace()).count();
        self.total += kept;
        self.link += if link { kept } else { 0 };
        if pre {
            self.text.push_str(&text.replace('\r', ""));
            return;
        }
        for ch in text.chars() {
            if !ch.is_whitespace() {
                self.text.push(ch);
            } else if !self.text.is_empty() && !self.text.ends_with([' ', '\n']) {
                self.text.push(' ');
            }
        }
    }
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

    const GEMA: &str = r#"<html><head><title>KJ 004. Hai Mari Sembah | GEMA</title><script>var x = "Baris palsu satu<br>dua";</script></head><body>
<nav><a href="/">Beranda</a><a href="/daftar_lirik">Daftar Lirik</a><a>Kidung</a><a>Kontak</a></nav>
<div class="field field--name-body field--label-hidden"><p><span> [ <i><b><a href="/x">Mzm 104</a></b></i> ]<br><br />
<b>1</b>. Baris satu contoh<br>Baris dua contoh<br>Baris tiga contoh<br>Baris empat contoh<br><br />
<b>2</b>. Bait dua baris satu<br>Bait dua baris dua<br>Bait dua baris tiga<br>Bait dua baris empat</p>
<div>[ <i><a href="/y">Kej 1:3-31</a></i> ]</div>
<p><b>3</b>. Bait tiga baris satu<br>Bait tiga baris dua</p></div>
<div class="field field--name-body"><p>Promo<br>Acara<br>Lain<br>Sekali</p></div>
<div class="field field--name-field-kategori"><div class="field__label">Kategori</div>Pujian</div>
<footer>Hak cipta &copy; 2006 YLSA.</footer></body></html>"#;

    #[test]
    fn a_gema_page_gives_its_title_and_numbered_stanzas_without_numbers_or_references() {
        let song = words_of(GEMA, "https://gema.sabda.org/kj_004_hai_mari_sembah").unwrap();
        assert_eq!(song.title, "KJ 004. Hai Mari Sembah");
        assert_eq!(song.source, "gema.sabda.org");
        assert_eq!(
            song.text,
            "Baris satu contoh\nBaris dua contoh\nBaris tiga contoh\nBaris empat contoh\n\nBait dua baris satu\nBait dua baris dua\nBait dua baris tiga\nBait dua baris empat\n\nBait tiga baris satu\nBait tiga baris dua"
        );
    }

    #[test]
    fn a_kapanlagi_page_keeps_its_line_breaks_and_stanzas() {
        let html = r#"<title>Lirik Lagu Contoh - Terima Contoh</title><div class="menu"><a>Satu</a></div>
<div class="col-lirik lyric-body lyrics-detail"><span class="lirik-line">Oh baris satu</span><br><span class="lirik-line">Oh baris dua</span><br><br>
<span class="lirik-line">Bait dua satu</span><br><span class="lirik-line">Bait dua dua</span></div><div class="lyrics-story">Cerita tentang lagu ini panjang sekali.</div>"#;
        let song = words_of(html, "https://lirik.kapanlagi.com/artis/contoh/terima-contoh/").unwrap();
        assert_eq!(song.text, "Oh baris satu\nOh baris dua\n\nBait dua satu\nBait dua dua");
    }

    #[test]
    fn a_chordtela_page_loses_its_chord_lines_and_meta() {
        let html = r#"<div class="post-body entry-content"><div class="adstela">Iklan</div><pre><b>Capo 2nd fret</b>

            <b class="tbi-tooltip">C<span class="custom tbi-C"></span></b>        <b class="tbi-tooltip">Cmaj7<span class="custom tbi-C"></span></b>
baris satu kata
<b class="tbi-tooltip">C7<span class="custom tbi-C7"></span></b>             <b class="tbi-tooltip">Am<span class="custom tbi-Am"></span></b>
 baris dua kata

Chorus :
 <b class="tbi-tooltip">F<span class="custom"></span></b>     <b class="tbi-tooltip">G<span class="custom"></span></b>
  baris tiga kata
 baris empat kata
Intro : G D Am C
(2x)
===ORIGINAL CHORD===
salinan satu
salinan dua
</pre></div>"#;
        let song = words_of(html, "https://www.chordtela.com/2026/11/contoh.html").unwrap();
        assert_eq!(song.text, "baris satu kata\nbaris dua kata\n\n[Chorus]\nbaris tiga kata\nbaris empat kata");
    }

    #[test]
    fn a_genius_page_drops_its_header_and_excluded_nodes() {
        let html = r#"<div data-lyrics-container="true" class="Lyrics__Container"><div class="LyricsHeader__Container">3 Contributors<br>Judul Lirik</div>[Verse 1: Seseorang]<br>Baris satu<br>Baris dua<br><span data-exclude-from-selection="true">Iklan di tengah</span>Baris tiga<br><br>[Chorus]<br>Reff satu<br>Reff dua</div>
<div data-lyrics-container="true">Reff tiga<br>Reff empat</div>"#;
        let song = words_of(html, "https://genius.com/contoh-lyrics").unwrap();
        assert_eq!(song.text, "[Verse 1]\nBaris satu\nBaris dua\nBaris tiga\n\n[Chorus]\nReff satu\nReff dua\nReff tiga\nReff empat");
    }

    #[test]
    fn a_letras_page_makes_a_stanza_of_each_paragraph() {
        let html = r#"<div class="lyric-original"><p>Satu baris<br>Dua baris</p><p>Tiga baris<br>Empat baris</p></div><div class="lyric-translation"><p>Lain<br>Sekali</p></div>"#;
        let song = words_of(html, "https://www.letras.mus.br/contoh/lagu/").unwrap();
        assert_eq!(song.text, "Satu baris\nDua baris\n\nTiga baris\nEmpat baris");
    }

    #[test]
    fn an_azlyrics_page_reads_the_unclassed_div() {
        let html = r#"<div class="col-xs-12 col-lg-8 text-center"><div class="ringtone">tidak</div><b>"Judul"</b><div><!-- Usage of azlyrics.com content --><br>
Baris satu<br>Baris dua<br><br>Baris tiga<br>Baris empat<br></div><div class="lyricsh">Lain</div><div id="x">Lain lagi dengan banyak kata kata kata kata kata</div></div>"#;
        let song = words_of(html, "https://www.azlyrics.com/lyrics/contoh/lagu.html").unwrap();
        assert_eq!(song.text, "Baris satu\nBaris dua\n\nBaris tiga\nBaris empat");
    }

    #[test]
    fn json_ld_lyrics_are_read_when_no_selector_knows_the_site() {
        let html = r#"<script type="application/ld+json">{"@graph":[{"@type":"WebPage"},{"@type":"MusicComposition","name":"x","lyrics":{"@type":"CreativeWork","text":"Baris satu\r\nBaris dua\r\n\r\nBaris tiga"}}]}</script><p>Tidak</p>"#;
        let song = words_of(html, "https://contoh.example/lagu").unwrap();
        assert_eq!(song.text, "Baris satu\nBaris dua\n\nBaris tiga");
    }

    #[test]
    fn a_blog_page_gives_the_song_and_not_the_article_or_the_menu() {
        let html = format!(
            "<title>Lirik Lagu Contoh Pujian - Blog Contoh</title><div class=\"sidebar\"><ul><li>Satu</li><li>Dua</li><li>Tiga</li><li>Empat</li><li>Lima</li><li>Enam</li></ul></div><p>{}</p><div class=\"entry\">Contoh baris satu<br>Contoh baris dua<br>Contoh baris tiga</div><div>Reff:<br>Contoh reff satu<br>Contoh reff dua</div><p>{}</p>",
            "Artikel ini panjang sekali dan menjelaskan latar belakang lagu ini dengan banyak kalimat yang tidak dinyanyikan sama sekali.",
            "Baca juga: artikel lain yang sangat panjang dengan banyak kalimat dan tautan ke halaman lain di situs ini."
        );
        let song = words_of(&html, "https://www.contoh.example/lirik").unwrap();
        assert_eq!(song.title, "Contoh Pujian");
        assert_eq!(song.source, "contoh.example");
        assert_eq!(song.text, "Contoh baris satu\nContoh baris dua\nContoh baris tiga\n\n[Reff]\nContoh reff satu\nContoh reff dua");
    }

    #[test]
    fn a_site_selector_that_finds_nothing_falls_through_to_the_shape() {
        let html = "<div class=\"post\">Baris satu kata<br>Baris dua kata<br>Baris tiga kata<br>Baris empat kata</div>";
        let song = words_of(html, "https://gema.sabda.org/lain").unwrap();
        assert_eq!(song.text, "Baris satu kata\nBaris dua kata\nBaris tiga kata\nBaris empat kata");
    }

    #[test]
    fn a_page_with_no_song_is_not_found() {
        let html = "<title>Beranda</title><p>Satu kalimat panjang yang tidak berbentuk lagu sama sekali karena hanya prosa biasa saja di sini.</p>";
        assert_eq!(words_of(html, "https://contoh.example/").unwrap_err(), LyricsError::NotFound);
    }

    #[test]
    fn each_site_gives_the_song_name_and_the_artist() {
        let title = |html: &str, url: &str| page_title(&Html::parse_document(html), &site_of(url));
        let pair = |song: &str, artist: &str| (song.to_string(), artist.to_string());
        let chordtela = r#"<title>Kunci Gitar Contoh Band - Lagu Contoh Chord Dasar ©ChordTela.com</title><meta property="og:title" content="Kunci Gitar Contoh Band - Lagu Contoh Chord Dasar ©ChordTela.com">"#;
        assert_eq!(title(chordtela, "https://www.chordtela.com/2026/11/x.html"), pair("Lagu Contoh", "Contoh Band"));
        let genius = "<title>Contoh\u{a0}Band & Tamu – Lagu Contoh Lyrics | Genius Lyrics</title><meta property=\"og:title\" content=\"Contoh\u{a0}Band (Ft. Tamu) – Lagu Contoh\">";
        assert_eq!(title(genius, "https://genius.com/x-lyrics"), pair("Lagu Contoh", "Contoh Band (Ft. Tamu)"));
        let azlyrics = r#"<title>Contoh Band - Lagu Contoh Lyrics | AZLyrics.com</title><h1>"Lagu Contoh" lyrics</h1>"#;
        assert_eq!(title(azlyrics, "https://www.azlyrics.com/lyrics/x/y.html"), pair("Lagu Contoh", "Contoh Band"));
        let lirik = r#"<title>Lirik Lagu Contoh - Contoh Band | Makna dan Arti Lagu</title><meta property="og:title" content="Lirik Lagu Contoh - Contoh Band | Makna dan Arti Lagu">"#;
        assert_eq!(title(lirik, "https://lirik.kapanlagi.com/artis/x/y/"), pair("Lagu Contoh", "Contoh Band"));
        let terjemahan = r#"<meta property="og:title" content="Terjemahan Lagu Contoh - Contoh Band | Arti Lirik">"#;
        assert_eq!(title(terjemahan, "https://lirik.kapanlagi.com/artis/x/y-terjemahan/"), pair("Lagu Contoh", "Contoh Band"));
        // A page without the structured source falls back to the cut title.
        assert_eq!(title("<title>Lirik Lagu Contoh - Contoh Band | Situs</title>", "https://www.azlyrics.com/a"), pair("Contoh", ""));
    }

    #[test]
    fn entities_and_addresses_are_read() {
        assert_eq!(decode("&#039;Ku Tak Dapat &amp; &#x41;&nbsp;b &bogus x"), "'Ku Tak Dapat & A b &bogus x");
        assert_eq!(site_of("https://www.example.org/a?b"), "example.org");
        assert!(from_page("gema.sabda.org/kj").is_err());
        let doc = Html::parse_document("<title>Lagu &amp; Puji | Situs</title>");
        assert_eq!(page_title(&doc, "x"), ("Lagu & Puji".to_string(), String::new()));
    }
}
