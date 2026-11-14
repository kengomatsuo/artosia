//! The pure rules the poster styles fit text by: balanced wrapping, the name ladder, money.

/// A token of a verse line: a word, or a verse number set raised in the same line.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Word(String),
    Num(String),
}

/// Greedy wrap of `toks` to `width`; `word` measures a word, `num` a raised number with its gap.
pub fn wrap(toks: &[Tok], width: f32, space: f32, word: &dyn Fn(&str) -> f32, num: &dyn Fn(&str) -> f32) -> Vec<Vec<Tok>> {
    let mut lines: Vec<Vec<Tok>> = Vec::new();
    let (mut line, mut used) = (Vec::new(), 0.0_f32);
    for tok in toks {
        let (w, glue) = match tok {
            Tok::Word(text) => (word(text), if line.is_empty() { 0.0 } else { space }),
            Tok::Num(text) => (num(text), 0.0),
        };
        if !line.is_empty() && used + w + glue > width {
            lines.push(std::mem::take(&mut line));
            used = 0.0;
            line.push(tok.clone());
            used += w;
        } else {
            line.push(tok.clone());
            used += w + glue;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The greedy wrap narrowed to the least width that keeps its line count: lines of near-equal length.
pub fn balanced(toks: &[Tok], width: f32, space: f32, word: &dyn Fn(&str) -> f32, num: &dyn Fn(&str) -> f32) -> (Vec<Vec<Tok>>, f32) {
    let count = wrap(toks, width, space, word, num).len();
    let (mut lo, mut hi) = (width * 0.5, width);
    for _ in 0..18 {
        let mid = (lo + hi) / 2.0;
        if wrap(toks, mid, space, word, num).len() <= count {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (wrap(toks, hi, space, word, num), hi)
}

/// Plain words as tokens.
pub fn words(text: &str) -> Vec<Tok> {
    text.split(' ').filter(|word| !word.is_empty()).map(|word| Tok::Word(word.to_string())).collect()
}

/// A line of tokens as one string, the numbers dropped.
pub fn join(line: &[Tok]) -> String {
    line.iter().filter_map(|tok| if let Tok::Word(word) = tok { Some(word.as_str()) } else { None }).collect::<Vec<_>>().join(" ")
}

const HONORIFICS: [&str; 6] = ["Bpk.", "Ibu", "Sdr.", "Sdri.", "Ny.", "Adk."];

/// Middle names to their initial: "Ny. Debora Anastasia Nainggolan" is "Ny. Debora A. Nainggolan".
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    let head = usize::from(parts.first().is_some_and(|first| HONORIFICS.contains(first)));
    let rest = &parts[head..];
    if rest.len() <= 2 {
        return name.to_string();
    }
    let mut out: Vec<String> = parts[..head].iter().map(|part| part.to_string()).collect();
    out.push(rest[0].to_string());
    out.extend(rest[1..rest.len() - 1].iter().map(|part| format!("{}.", part.chars().next().unwrap_or(' '))));
    out.push(rest[rest.len() - 1].to_string());
    out.join(" ")
}

/// The name as it fits `width`: whole, else with middle names as initials; None when neither fits.
pub fn ladder(name: &str, width: f32, measure: &dyn Fn(&str) -> f32) -> Option<String> {
    [name.to_string(), initials(name)].into_iter().find(|candidate| measure(candidate) <= width)
}

/// `12345678` as `12.345.678`, the church's one style of digits.
pub fn digits(value: i64) -> String {
    let plain = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (at, ch) in plain.chars().enumerate() {
        if at > 0 && (plain.len() - at).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    if value < 0 { format!("-{out}") } else { out }
}

/// `Rp 12.345.678`.
pub fn rupiah(value: i64) -> String {
    format!("Rp {}", digits(value))
}

/// The amount of a cell such as "Rp 1.500.000": its digits as a number, 0 when it has none.
pub fn amount(cell: &str) -> i64 {
    cell.chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str) -> f32 {
        text.chars().count() as f32 * 10.0
    }

    fn num(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    #[test]
    fn balanced_lines_keep_the_greedy_count_and_even_out() {
        let toks = words("aaaa bbbb cccc dddd eeee ffff");
        let greedy = wrap(&toks, 140.0, 10.0, &word, &num);
        assert_eq!(greedy.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3]);
        let (even, width) = balanced(&words("aaaaaaaaaa b cc ddddddd e"), 200.0, 10.0, &word, &num);
        let longest = even.iter().map(|line| join(line).len()).max().unwrap_or(0);
        assert!(even.len() == wrap(&words("aaaaaaaaaa b cc ddddddd e"), 200.0, 10.0, &word, &num).len() && width <= 200.0 && longest <= 18);
    }

    #[test]
    fn a_verse_number_stays_with_the_words_that_follow_it_on_a_line() {
        let toks = vec![Tok::Num("1".into()), Tok::Word("Aku".into()), Tok::Word("datang".into()), Tok::Num("2".into()), Tok::Word("pergi".into())];
        let lines = wrap(&toks, 100.0, 10.0, &word, &num);
        assert_eq!(lines.concat(), toks);
        assert!(lines.iter().all(|line| line.iter().map(|tok| match tok { Tok::Word(w) => word(w), Tok::Num(n) => num(n) }).sum::<f32>() <= 120.0));
    }

    #[test]
    fn the_ladder_shortens_middle_names_and_keeps_the_honorific() {
        assert_eq!(initials("Ny. Debora Anastasia Nainggolan"), "Ny. Debora A. Nainggolan");
        assert_eq!(initials("Bpk. Paulus Setiabudi Wibowo"), "Bpk. Paulus S. Wibowo");
        assert_eq!(initials("Ibu Susi Handayani"), "Ibu Susi Handayani");
        assert_eq!(initials("Adk. Keren"), "Adk. Keren");
        assert_eq!(ladder("Bpk. Paulus Setiabudi Wibowo", 250.0, &word).as_deref(), Some("Bpk. Paulus S. Wibowo"));
        assert_eq!(ladder("Bpk. Paulus Setiabudi Wibowo", 10.0, &word), None);
        assert_eq!(ladder("Ibu Hana", 100.0, &word).as_deref(), Some("Ibu Hana"));
    }

    #[test]
    fn money_has_dot_thousands_and_a_total_never_loses_a_digit() {
        assert_eq!(rupiah(1_234_567_890), "Rp 1.234.567.890");
        assert_eq!(digits(999), "999");
        assert_eq!(digits(1000), "1.000");
        assert_eq!(rupiah(281_598_609_247), "Rp 281.598.609.247");
        assert_eq!(amount("Rp 1.500.000"), 1_500_000);
        assert_eq!(amount("none"), 0);
    }
}
