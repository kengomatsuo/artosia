//! Direction 7, Swiss numerals: a flush-left grid where the date is the biggest thing, set in the one dark column, rows on hairlines and one accent.

use artosia_core::deck::NoticeKind;

use super::grown_size;
use super::data::{Data, Have};
use super::fit;
use super::{Fits, LINE, Pic, Pos, R, Sheet, Split, Tm, Tx, VerseBox, fit_cells, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

pub fn draw(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    match d.kind {
        NoticeKind::Schedule => s01(sl, d),
        NoticeKind::Birthdays => s02(sl, d),
        NoticeKind::Money => s03(sl, d),
        NoticeKind::Account => s04(sl, d, have),
        NoticeKind::Verse => s05(sl, d),
        NoticeKind::Plain => s06(sl, d, have),
    }
}

/// The part of a long card, centred in the strip at the foot.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::c((sl.g("x2") + sl.g("mx")) / 2.0, sl.g("kicker_y")));
}

/// The ground and the strip at the foot: the warta at its left and `right`, else the church, at its right.
fn chrome(sl: &mut Sheet, d: &Data, right: Option<&str>) {
    let (mx, x2, ky) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_y"));
    sl.push(R::new(0.0, 0.0, super::W, super::H).fill("paper"));
    sl.push(R::new(mx, sl.g("strip_y"), x2 - mx, sl.g("strip_w")).fill("ink"));
    sl.text("kicker", &d.w.warta, Tx::l(mx, ky));
    let right = right.filter(|words| !words.is_empty()).unwrap_or(&d.church);
    if !right.is_empty() {
        sl.text("kicker", right, Tx::r(x2, ky));
    }
}

/// The title in two tones: its first word light and muted, the rest bold and dark, as large as `room` lets it be.
fn title(sl: &mut Sheet, text: &str, room: f32) -> Tm {
    let (mx, base) = (sl.g("mx"), sl.g("title_base"));
    let (first, rest) = text.split_once(' ').map_or((text, ""), |(first, rest)| (first, rest.trim()));
    // A lone word stays bold; with more, the first is the light muted one.
    let (face, tone) = if rest.is_empty() { (None, "ink") } else { (Some("l"), "mute") };
    let width = |sl: &Sheet, z: f32| sl.measure("title", first, Some(z), face) + if rest.is_empty() { 0.0 } else { sl.measure("title", " ", Some(z), None) + sl.measure("title", rest, Some(z), None) };
    let z = largest(sl.ty_size("title"), 64.0, |z| width(sl, z) <= room).unwrap_or(64.0);
    let mut head = Tx::l(mx, base).size(z).fill(tone);
    if let Some(face) = face {
        head = head.font(face);
    }
    let a = sl.text("title", first, head);
    if rest.is_empty() {
        return a;
    }
    let at = a.x + a.w + sl.measure("title", " ", Some(z), None);
    let b = sl.text("title", rest, Tx::l(at, base).size(z));
    Tm { x: a.x, w: b.x + b.w - a.x }
}

/// A hairline `y` is the middle of.
fn hair(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let height = sl.g("hair");
    sl.push(R::new(x, y - height / 2.0, width, height).fill("rule"));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    chrome(sl, d, None);
    let week = d.week_short.as_str();
    let week_w = if week.is_empty() { 0.0 } else { sl.measure("subtitle", week, None, None) + 48.0 };
    title(sl, &d.title, x2 - mx - week_w);
    if !week.is_empty() {
        sl.text("subtitle", week, Tx::r(x2, base));
    }
    // Rows are grouped by day: one dark block per group.
    let mut sizes: Vec<usize> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match sizes.last_mut() {
            Some(n) if i > 0 && r.day == rows[i - 1].day && r.date == rows[i - 1].date => *n += 1,
            _ => sizes.push(1),
        }
    }
    let avail = bottom - top;
    let (event_x, time_x, block_w, ratio) = (k("event_x"), k("time_x"), k("block_w"), k("ratio"));
    let height = |z: f32, extra: f32| sizes.iter().map(|n| (*n as f32 * (ratio * z + extra)).max(k("group_min"))).sum::<f32>() + sizes.len().saturating_sub(1) as f32 * k("group_gap");
    // A special row keeps room at its right for the word that names it, so it stays on one line.
    let tagged = x2 - event_x - k("tag_room");
    let tags_fit = |z: f32| rows.iter().filter(|r| r.special).all(|r| sl.measure("event_hl", &r.event, Some(z), None) <= tagged);
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event", &events, |_| x2 - event_x - 24.0, k("size_max"), k("size_floor"), |z, e| height(z, e) <= avail && tags_fit(z)).ok_or(Split)?;
    let pitch = ratio * size + extra;
    let (fig_z, day_z) = (sl.ty_size("fig"), sl.ty_size("day_name"));
    let bx = mx + block_w;
    let (mut y, mut at) = (top, 0);
    for n in sizes {
        let gh = (n as f32 * pitch).max(k("group_min"));
        sl.push(R::new(mx, y, block_w, gh).fill("block"));
        for j in 0..n {
            let ry = y + j as f32 * pitch;
            if rows[at + j].special {
                sl.push(R::new(bx, ry, x2 - bx, pitch).fill("accent"));
            } else {
                hair(sl, bx, if j + 1 < n { ry + pitch } else { y + gh }, x2 - bx);
            }
        }
        let first = &rows[at];
        let numeral = if first.date > 0 { first.date.to_string() } else { first.dow.clone() };
        let figure = y + 22.0 + 0.7 * fig_z;
        sl.text("fig", &numeral, Tx::l(mx + 24.0, figure).maxw(block_w - 48.0));
        sl.text("day_name", &first.day, Tx::l(mx + 24.0, figure + 18.0 + 0.72 * day_z).maxw(block_w - 48.0));
        for j in 0..n {
            let (r, cell) = (&rows[at + j], &cells[at + j]);
            let mid = y + j as f32 * pitch + pitch / 2.0;
            let (time, event) = if r.special { ("time_hl", "event_hl") } else { ("time", "event") };
            sl.text(time, &r.time, Tx::l(time_x, mid - (cell.len() as f32 - 1.0) / 2.0 * LINE * size).cy().size(size).maxw(event_x - time_x - 12.0));
            put_lines(sl, event, cell, event_x, mid, size, x2 - event_x - 24.0);
            if r.special {
                sl.text("tag", &d.w.special, Tx::r(x2 - 24.0, mid).cy());
            }
        }
        at += n;
        y += gh + k("group_gap");
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    chrome(sl, d, None);
    let month_w = if d.month.is_empty() { 0.0 } else { sl.measure("subtitle", &d.month, None, None) + 40.0 };
    let range_w = if d.highlighted() { sl.measure("ribbon", &d.week_label, None, None) + 24.0 } else { 0.0 };
    let t = title(sl, &d.title, x2 - mx - month_w - range_w);
    sl.need(t.w + month_w + range_w <= x2 - mx)?;
    if !d.month.is_empty() {
        sl.text("subtitle", &d.month, Tx::l(t.x + t.w + 40.0, base));
    }
    if d.highlighted() {
        sl.text("ribbon", &d.week_label, Tx::r(x2, base));
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let (colw, avail, ratio) = (((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32, bottom - top, k("ratio"));
    // The day is set at the name's size, so the room a name has shrinks as the size grows.
    let mon_w = if d.spans { names.iter().map(|x| sl.measure("bday_mon", &x.month, None, None)).fold(0.0, f32::max) } else { 0.0 };
    let day_cell = |sl: &Sheet, z: f32| sl.measure("bday_day_hl", "00", Some(z), None) + if d.spans { k("day_gap") + mon_w } else { 0.0 };
    let name_room = |sl: &Sheet, z: f32| colw - day_cell(sl, z) - k("name_gap");
    let hi = grown_size(k("size_max"), k("size_max") * 1.7, avail, per, ratio);
    let fits = |z: f32| names.iter().all(|nm| fit::ladder(&nm.name, name_room(sl, z), &|s| sl.measure("name_hl", s, Some(z), None)).is_some());
    let size = largest(hi, k("size_floor"), fits).ok_or(Split)?;
    let pitch = (avail / per as f32).min(ratio * 1.35 * size);
    let (cell, room, num_w) = (day_cell(sl, size), name_room(sl, size), sl.measure("bday_day_hl", "00", Some(size), None));
    let cx_of = |c: usize| mx + c as f32 * (colw + k("col_gap"));
    for i in 0..names.len() {
        hair(sl, cx_of(i / per), top + (i % per) as f32 * pitch + pitch, colw);
    }
    for c in 0..cols {
        let run: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).map(|i| i - c * per).collect();
        if let (Some(first), Some(last)) = (run.iter().min(), run.iter().max()) {
            sl.push(R::new(cx_of(c) - k("band_over"), top + *first as f32 * pitch, colw + 2.0 * k("band_over"), (last - first + 1) as f32 * pitch).fill("accent"));
        }
    }
    let cap = sl.cap(&sl.face_of("name"), size);
    for (i, nm) in names.iter().enumerate() {
        let (cx, by) = (cx_of(i / per), top + (i % per) as f32 * pitch + (pitch + cap) / 2.0);
        let (day, mon, role) = if nm.hl { ("bday_day_hl", "bday_mon_hl", "name_hl") } else { ("bday_day", "bday_mon", "name") };
        let (text, _) = name_in(sl, &nm.name, role, room, size);
        sl.text(day, &format!("{:02}", nm.day), Tx::r(cx + num_w, by).size(size));
        if d.spans {
            sl.text(mon, &nm.month, Tx::l(cx + num_w + k("day_gap"), by));
        }
        sl.text(role, &text, Tx::l(cx + cell + k("name_gap"), by).size(size).maxw(room));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    chrome(sl, d, Some(&d.w.offerings));
    let date = d.date_long.as_deref().unwrap_or("");
    title(sl, &d.title, x2 - mx - if date.is_empty() { 0.0 } else { sl.measure("subtitle", date, None, None) + 48.0 });
    if !date.is_empty() {
        sl.text("subtitle", date, Tx::r(x2, base));
    }
    let total_y = bottom - k("total_h");
    let avail = total_y - k("total_gap") - top;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let amount_w = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let label_room = |sl: &Sheet, z: f32| x2 - mx - rp_w - k("rp_gap") - amount_w(sl, z) - 48.0;
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, |z| label_room(sl, z), k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let (amount_w, label_w) = (amount_w(sl, size), label_room(sl, size));
    let pitch = (avail / n as f32).min(k("ratio_max") * size + extra);
    let cap = sl.cap(&sl.face_of("fund"), size);
    for i in 0..funds.len() {
        hair(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx);
    }
    for (i, f) in funds.iter().enumerate() {
        let cy = top + i as f32 * pitch + pitch / 2.0;
        put_lines(sl, "fund", &cells[i], mx, cy, size, label_w);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, cy + cap / 2.0).size(size));
        sl.text("rp", "Rp", Tx::r(x2 - amount_w - k("rp_gap"), cy + cap / 2.0));
    }
    sl.push(R::new(mx, total_y, x2 - mx, 2.0).fill("ink"));
    let sum = fit::digits(d.total());
    let rp_w = sl.measure("total_rp", "Rp", None, None);
    let z = sl.must(largest(sl.ty_size("total"), 96.0, |z| sl.measure("total", &sum, Some(z), None) + rp_w + 24.0 <= x2 - mx), 96.0)?;
    let width = sl.measure("total", &sum, Some(z), None);
    let by = total_y + 56.0 + 30.0 + 0.72 * z;
    sl.text("total_label", &d.w.total_gifts, Tx::l(mx, total_y + 56.0));
    sl.text("total", &sum, Tx::r(x2, by).size(z));
    sl.text("total_rp", "Rp", Tx::r(x2 - width - 24.0, by));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s04");
    chrome(sl, d, Some(&d.title));
    let wide = if have.qris { x2 - mx - k("card") - k("col_gap") } else { x2 - mx };
    let a = &d.account;
    let nsz = sl.must(largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide), k("number_floor"))?;
    let hsz = sl.must(largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide), k("holder_floor"))?;
    let y = k("stack_top");
    sl.text("bank", &a.bank, Tx::l(mx, y).maxw(wide));
    let nb = y + 36.0 + 0.72 * nsz;
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    sl.push(R::new(mx, nb + 36.0, wide, 2.0).fill("ink"));
    sl.text("atas", &d.w.holder, Tx::l(mx, nb + 36.0 + 60.0));
    sl.text("holder", &a.holder, Tx::l(mx, nb + 36.0 + 60.0 + 72.0).size(hsz).maxw(wide));
    let mut ny = nb + 36.0 + 60.0 + 72.0 + 64.0;
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), sl.ty_size("note"), wide) };
    for line in &notes {
        sl.text("note", line, Tx::l(mx, ny).maxw(wide));
        ny += 54.0;
    }
    // The notes stay above the strip at the foot.
    sl.need(notes.is_empty() || ny - 54.0 <= bottom)?;
    if have.qris {
        // The code is dark on light, so it keeps a white plate in every colourway.
        let (px, py, pad) = (x2 - k("card"), k("qr_top"), k("card_pad"));
        sl.push(R::new(px, py, k("card"), k("card")).fill("card"));
        sl.image(Pic::Qris, px + pad, py + pad, k("card") - 2.0 * pad, k("card") - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(px + k("card") / 2.0, py + k("card") + 56.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    chrome(sl, d, d.date_long.as_deref());
    sl.text("caption", &d.w.verse, Tx::l(mx, k("ref_y") - 120.0));
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(k("text_w")));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    chrome(sl, d, d.date_long.as_deref());
    let wide = if have.photo { x2 - mx - k("photo_w") - k("photo_gap") } else { k("wide") };
    if have.photo {
        let (px, py, ph) = (x2 - k("photo_w"), k("photo_top"), k("photo_h"));
        if have.photo2 {
            let half = (ph - 24.0) / 2.0;
            sl.image(Pic::Photo, px, py, k("photo_w"), half, 0.0, false, Pos::Mid);
            sl.image(Pic::Photo2, px, py + half + 24.0, k("photo_w"), half, 0.0, false, Pos::Mid);
        } else {
            sl.image(Pic::Photo, px, py, k("photo_w"), ph, 0.0, false, Pos::Mid);
        }
    }
    sl.text("caption", &d.w.notice, Tx::l(mx, k("kick_y")));
    let (ls, face) = (sl.ty("n_title").ls, sl.face_of("n_title"));
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::balanced(&words, wide * 0.94, space, &word, &word).0.iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 2), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), 0.72 * z);
    // The last line of the title is the bold one.
    for (i, line) in lines.iter().enumerate() {
        let at = Tx::l(mx, k("top") + cap + i as f32 * tl).size(z).maxw(wide);
        sl.text("n_title", line, if i + 1 == lines.len() { at.font("b") } else { at });
    }
    let mut y = k("top") + cap + lines.len().saturating_sub(1) as f32 * tl + 96.0;
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.375).round();
    for para in d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect::<Vec<_>>() {
        for line in &para {
            sl.text("n_body", line, Tx::l(mx, y).maxw(wide));
            y += blh;
        }
        y += k("line_gap");
    }
    // A body taller than the page does not fit at this size.
    sl.need(y - k("line_gap") - blh <= bottom)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn birthdays(count: usize) -> Slide {
        let text: Vec<String> = (0..count).map(|at| format!("Ibu Contoh Nama Lengkap {at}\t{} November", at * 28 / count.max(1) + 1)).collect();
        Slide { notice: NoticeKind::Birthdays, heading: "Ulang Tahun".into(), text: text.join("\n"), marked: (7..14.min(count)).collect(), ..Slide::default() }
    }

    #[test]
    fn thirty_names_are_one_slide_and_more_continue() {
        assert_eq!(paginate(&birthdays(30), NoticeStyle::SwissNumerals).len(), 1);
        assert!(paginate(&birthdays(60), NoticeStyle::SwissNumerals).len() > 1);
    }

    #[test]
    fn a_week_of_eight_rows_in_four_days_is_one_slide() {
        let text = "Minggu\t07.00\tIbadah Raya I\n\t09.30\tIbadah Raya II dan Sekolah Minggu\nRabu\t18.30\tPersekutuan Doa dan Pemahaman Alkitab\n\t20.00\tLatihan Paduan Suara dan Tim Pemusik\nJumat\t17.30\tPelayanan Kunjungan Kasih ke Rumah Sakit\n\t19.00\tIbadah Doa Puasa Bersama Seluruh Jemaat\nSabtu\t09.00\tRapat Majelis dan Pengurus Komisi Anak\n\t16.00\tBakti Sosial dan Pembagian Sembako Jemaat";
        let slide = Slide { notice: NoticeKind::Schedule, heading: "Jadwal Pekan Ini".into(), text: text.into(), marked: vec![5], ..Slide::default() };
        assert_eq!(paginate(&slide, NoticeStyle::SwissNumerals).len(), 1);
    }
}
