//! Direction 8, Quiet tint: one sans in two weights on a tinted ground, a hairline under the title and one solid pill under whatever is special.

use artosia_core::deck::NoticeKind;

use super::grown_size;
use super::data::{Data, Have};
use super::fit;
use super::{Fits, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The ground, the kicker line (`right` at its right) and the hairline across the page.
fn page(sl: &mut Sheet, d: &Data, right: &str) {
    let (mx, x2, ky) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_y"));
    sl.push(R::new(0.0, 0.0, W, 1080.0).fill("paper"));
    let taken = if right.is_empty() { 0.0 } else { sl.text("kicker", right, Tx::r(x2, ky).maxw((x2 - mx) / 2.0)).w + 40.0 };
    sl.text("kicker", &d.w.warta, Tx::l(mx, ky).maxw(x2 - mx - taken));
    let (y, h) = (sl.g("rule_y"), sl.g("rule_w"));
    sl.push(R::new(0.0, y, W, h).fill("rule"));
}

/// The title with its first word light and muted and the rest bold; a one-word title is bold. Returns where it ends.
fn masthead(sl: &mut Sheet, title: &str) -> f32 {
    let (mx, x2, base) = (sl.g("mx"), sl.g("x2"), sl.g("title_base"));
    let Some((first, rest)) = title.trim().split_once(' ').map(|(first, rest)| (first, rest.trim())).filter(|(_, rest)| !rest.is_empty()) else {
        let t = sl.text("title", title.trim(), Tx::l(mx, base).maxw(x2 - mx));
        return t.x + t.w;
    };
    let a = sl.text("title_a", first, Tx::l(mx, base).maxw(x2 - mx));
    let at = a.x + a.w + sl.adv(&sl.face_of("title_a"), " ", sl.ty_size("title_a"), sl.ty("title_a").ls * sl.ty_size("title_a"));
    let b = sl.text("title", rest, Tx::l(at, base).maxw(x2 - at));
    b.x + b.w
}

/// `words` at the right of the title row, when they fit beside a title ending at `end`.
fn subtitle(sl: &mut Sheet, words: &str, end: f32) {
    let (x2, base) = (sl.g("x2"), sl.g("title_base"));
    if !words.is_empty() && end + 40.0 + sl.measure("subtitle", words, None, None) <= x2 {
        sl.text("subtitle", words, Tx::r(x2, base));
    }
}

/// The part of a long card, bold at the foot of the slide.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::l(sl.g("mx"), 1040.0));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, band_pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"));
    let (inset, k) = (sl.g("pill_inset"), sl.sec("s01"));
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, d, &d.church);
    let end = masthead(sl, &d.title);
    subtitle(sl, &d.week_short, end);
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let gaps = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32 * k("group_gap");
    let avail = bottom - top;
    let (day_x, date_x, time_x, event_x) = (k("day_x"), k("date_x"), k("time_x"), k("event_x"));
    let ev_w = x2 - event_x - if rows.iter().any(|r| r.special) { k("tag_room") } else { 0.0 };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) + gaps <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min((avail - gaps) / n as f32);
    let cap = sl.cap(&sl.face_of("event"), size);
    let day_size = (size * k("day_ratio")).min(k("day_max"));
    let day_cap = sl.cap(&sl.face_of("day_name"), day_size);
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if i > 0 && firsts[i] {
            y += k("group_gap");
        }
        let cy = y + pitch / 2.0;
        let by = cy + cap / 2.0;
        if r.special {
            sl.push(R::new(mx - band_pad, y + inset, x2 - mx + 2.0 * band_pad, pitch - 2.0 * inset).fill("pill").pill());
        }
        if firsts[i] {
            let on = |tx: Tx| if r.special { tx.fill("pill_ink") } else { tx };
            sl.text("day_name", &r.day, on(Tx::l(day_x, cy + day_cap / 2.0).size(day_size).maxw(date_x - day_x - 14.0)));
            if r.date > 0 {
                sl.text("date_small", &r.date.to_string(), on(Tx::l(date_x, cy + day_cap / 2.0).size(day_size).maxw(time_x - 20.0 - date_x)));
            }
        }
        sl.text(if r.special { "time_hl" } else { "time" }, &r.time, Tx::l(time_x, by).size(size).maxw(event_x - time_x - 20.0));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if r.special {
            sl.text("tag", &d.w.special, Tx::r(x2, cy).cy());
        }
        y += pitch;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let (inset, gap, k) = (sl.g("pill_inset"), sl.g("title_gap"), sl.sec("s02"));
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, d, &d.church);
    let end = masthead(sl, &d.title);
    let mut at = end + gap;
    if !d.month.is_empty() && at + sl.measure("subtitle", &d.month, None, None) <= x2 {
        at += sl.text("subtitle", &d.month, Tx::l(at, base)).w + 40.0;
    }
    if d.highlighted() && at + sl.measure("ribbon", &d.week_label, None, None) <= x2 {
        sl.text("ribbon", &d.week_label, Tx::r(x2, base));
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    // The day cell: the numerals right-aligned, then the month on a page that crosses one.
    let day_cell = |sl: &Sheet, z: f32| {
        let num = names.iter().map(|x| sl.measure("bday_day_hl", &format!("{:02}", x.day), Some(z), None)).fold(0.0, f32::max);
        let mon = if d.spans { names.iter().map(|x| sl.measure("bday_mon", &x.month, None, None)).fold(0.0, f32::max) + k("day_gap") } else { 0.0 };
        (num, num + mon)
    };
    let fits = |z: f32| {
        let name_w = colw - day_cell(sl, z).1 - k("name_gap");
        names.iter().all(|nm| fit::ladder(&nm.name, name_w, &|s| sl.measure(if nm.hl { "name_hl" } else { "name" }, s, Some(z), None)).is_some())
    };
    let size = largest(grown_size(k("size_max"), k("size_max") * 1.7, avail, per, 1.6), k("size_floor"), fits).ok_or(Split)?;
    let (num_w, day_w) = day_cell(sl, size);
    let name_w = colw - day_w - k("name_gap");
    let pitch = (avail / per as f32).min(k("ratio_max") * size);
    // One pill per run of highlighted names in a column, drawn first so the names sit on it.
    for c in 0..cols {
        let rows_in = n.saturating_sub(c * per).min(per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let mut r = 0;
        while r < rows_in {
            if !names[c * per + r].hl {
                r += 1;
                continue;
            }
            let start = r;
            while r < rows_in && names[c * per + r].hl {
                r += 1;
            }
            sl.push(R::new(cx - k("band_over"), top + start as f32 * pitch + inset, colw + 2.0 * k("band_over"), (r - start) as f32 * pitch - 2.0 * inset).fill("pill").rx(k("pill_rx")));
        }
    }
    let cap = sl.cap(&sl.face_of("name"), size);
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let by = top + r as f32 * pitch + pitch / 2.0 + cap / 2.0;
        let (role, day_role, mon_role) = if nm.hl { ("name_hl", "bday_day_hl", "bday_mon_hl") } else { ("name", "bday_day", "bday_mon") };
        sl.text(day_role, &format!("{:02}", nm.day), Tx::r(cx + num_w, by).size(size));
        if d.spans {
            sl.text(mon_role, &nm.month, Tx::l(cx + num_w + k("day_gap"), by));
        }
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(role, &text, Tx::l(cx + day_w + k("name_gap"), by).size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, band_pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, &d.w.offerings);
    let end = masthead(sl, &d.title);
    subtitle(sl, d.date_long.as_deref().unwrap_or(""), end);
    let avail = bottom - top - k("total_h") - k("total_gap");
    let rp_w = sl.measure("rp", "Rp", None, None);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let digits_at = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let lab_at = |z: f32| x2 - mx - (rp_w + k("rp_gap") + digits_at(sl, z)) - 40.0;
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, lab_at, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let amount_col = rp_w + k("rp_gap") + digits_at(sl, size);
    let label_w = x2 - mx - amount_col - 40.0;
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let cap = sl.cap(&sl.face_of("fund"), size);
    for (i, f) in funds.iter().enumerate() {
        let by = top + i as f32 * pitch + pitch / 2.0 + cap / 2.0;
        put_lines(sl, "fund", &cells[i], mx, by - cap / 2.0, size, label_w);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, by).size(size));
        sl.text("rp", "Rp", Tx::l(x2 - amount_col, by));
    }
    // The total is the one filled shape on the slide.
    let (ty, th, pad) = (bottom - k("total_h"), k("total_h"), k("total_pad"));
    sl.push(R::new(mx - band_pad, ty, x2 - mx + 2.0 * band_pad, th).fill("pill").pill());
    let cy = ty + th / 2.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(mx + pad, cy).cy());
    let rp = sl.measure("total_rp", "Rp", None, None) + k("rp_gap");
    let total = sl.text("total", &fit::digits(d.total()), Tx::r(x2 - pad, cy).cy().maxw(x2 - mx - 2.0 * pad - label.w - 48.0 - rp));
    sl.text("total_rp", "Rp", Tx::r(total.x - k("rp_gap"), cy).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s04");
    page(sl, d, &d.title);
    let wide = if have.qris { x2 - k("card") - k("col_gap") - mx } else { x2 - mx };
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let note_face = sl.face_of("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &note_face, sl.ty_size("note"), wide) };
    let top = k("stack_top");
    sl.text("bank", &a.bank, Tx::l(mx, top).maxw(wide));
    let nb = top + k("number_gap") + sl.cap(&sl.face_of("number"), nsz);
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    let ab = nb + k("atas_gap");
    sl.text("atas", &d.w.holder, Tx::l(mx, ab));
    let hb = ab + k("holder_gap");
    sl.text("holder", &a.holder, Tx::l(mx, hb).size(hsz).maxw(wide));
    for (i, line) in notes.iter().enumerate() {
        sl.text("note", line, Tx::l(mx, hb + k("note_gap") + i as f32 * k("note_pitch")).maxw(wide));
    }
    sl.need(hb + k("note_gap") + notes.len().saturating_sub(1) as f32 * k("note_pitch") <= bottom)?;
    if have.qris {
        // The code is dark on light, so it keeps a white plate on any ground.
        let (card, pad) = (k("card"), k("card_pad"));
        let plate = card + 2.0 * pad;
        let (cx, py) = (x2 - card, k("plate_y"));
        sl.push(R::new(cx - pad, py, plate, plate).fill("card").rx(k("plate_rx")));
        sl.image(Pic::Qris, cx, py + pad, card, card, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(cx - pad + plate / 2.0, py + plate + k("caption_gap")).maxw(plate + 120.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref().unwrap_or(""));
    sl.text("caption", &d.w.verse, Tx::l(mx, k("kicker_y")));
    if !d.reference.is_empty() {
        let pad = k("ref_pad");
        let w = sl.measure("ref", &d.reference, None, None).min(k("text_w"));
        sl.push(R::new(mx - pad, k("ref_y") - k("ref_up"), w + 2.0 * pad, k("ref_h")).fill("pill").pill());
        sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(k("text_w")));
    }
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, d, d.date_long.as_deref().unwrap_or(""));
    sl.text("caption", &d.w.notice, Tx::l(mx, k("kicker_y")));
    let wide = if have.photo { x2 - mx - k("photo_w") - k("photo_gap") } else { x2 - mx };
    if have.photo {
        sl.image(Pic::Photo, x2 - k("photo_w"), k("photo_y"), k("photo_w"), k("photo_h"), k("photo_rx"), false, Pos::Mid);
    }
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    // One line at the largest size that holds it, else two lines as large as they go.
    let (max, floor) = (k("title_max"), k("title_floor"));
    let z = match largest_step(max, floor, 1.0, |z| wrap_at(z).len() <= 1) {
        Some(z) => z,
        None => sl.must(largest_step(max, floor, 1.0, |z| wrap_at(z).len() <= 2), floor)?,
    };
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("title_top") + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let mut y = k("title_top") + cap + (lines.len() as f32 - 1.0) * tl + k("body_gap");
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * k("body_lh")).round();
    let mut last = y;
    for para in d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect::<Vec<_>>() {
        for line in &para {
            sl.text("n_body", line, Tx::l(mx, y).maxw(wide));
            last = y;
            y += blh;
        }
        y += k("line_gap");
    }
    // A body taller than the page does not fit at this size.
    sl.need(last <= bottom)?;
    Ok(())
}
