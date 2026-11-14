//! Direction 4, Split panel: a light text panel beside a navy panel that holds the thing itself.

use artosia_core::deck::NoticeKind;

use super::largest_step;
use super::data::{self as w, Data, Have};
use super::fit;
use super::{Fits, H, fit_cells, grown_size, put_lines, Pic, Pos, R, Sheet, Split, Tx, W, largest, largest_name_size, name_in, paragraph, paragraphs, verse, VerseBox};

pub fn draw(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    match d.kind {
        NoticeKind::Schedule => s01(sl, d),
        NoticeKind::Birthdays => s02(sl, d),
        NoticeKind::Money => s03(sl, d),
        NoticeKind::Account => s04(sl, d, have),
        NoticeKind::Verse => s05(sl, d, have),
        NoticeKind::Plain => s06(sl, d, have),
    }
}

fn ground(sl: &mut Sheet, left_w: Option<f32>, top_band: Option<f32>) {
    sl.push(R::new(0.0, 0.0, W, H).fill("light"));
    if let Some(left) = left_w {
        sl.push(R::new(left, 0.0, W - left, H).fill("navy"));
    }
    if let Some(band) = top_band {
        sl.push(R::new(0.0, 0.0, W, band).fill("navy"));
    }
}

fn dots(sl: &mut Sheet, x: f32, cy: f32, count: usize, dia: f32, gap: f32) {
    for k in 0..count {
        sl.push(R::new(x + k as f32 * (dia + gap), cy - dia / 2.0, dia, dia).fill("yellow").pill());
    }
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, ky, base, sub_base, top, bottom) = (sl.g("mx"), sl.g("kicker_y"), sl.g("title_base"), sl.g("sub_base"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    ground(sl, Some(k("left_w")), None);
    sl.text("kicker", &d.kicker(w::WIB), Tx::l(mx, ky).cy().maxw(900.0));
    sl.text("title", &d.title, Tx::l(mx, base).maxw(k("left_w") - 2.0 * mx));
    if !d.week_short.is_empty() {
        sl.text("sub", &d.week_short, Tx::l(mx, sub_base));
    }
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let breaks = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32;
    let avail = bottom - top;
    let sparse = n as f32 <= k("sparse_rows");
    let pitch = ((avail - breaks * k("group_gap")) / n as f32).min(if sparse { k("sparse_h") } else { 999.0 } + k("row_gap"));
    let rh = pitch - k("row_gap");
    let y0 = if sparse { top + (avail - (n as f32 * pitch + breaks * k("group_gap"))) / 2.0 } else { top };
    let (card_x, card_w) = (mx, k("left_w") - mx - 40.0);
    let time_x = card_x + k("badge_w") + k("time_pad");
    let ev_x = time_x + k("time_w") + 16.0;
    let ev_w = card_x + card_w - ev_x - 24.0;
    let hi = if sparse { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, _) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| z * 1.3 + e <= rh).ok_or(Split)?;
    let mut y = y0;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += k("group_gap");
        }
        let cy = y + rh / 2.0;
        sl.push(R::new(card_x, y, card_w, rh).fill(if r.special { "hl" } else { "white" }).rx(18.0));
        sl.push(R::new(card_x, y, k("badge_w"), rh).fill(if r.special { "red" } else { "blue" }).rx(18.0));
        let badge = if r.date > 0 { format!("{} {}", r.dow, r.date) } else { r.dow.clone() };
        sl.text("badge", &badge, Tx::c(card_x + k("badge_w") / 2.0, cy).cy().maxw(k("badge_w") - 24.0));
        sl.text("time", &r.time, Tx::l(time_x, cy).cy().maxw(k("time_w")));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], ev_x, cy, size, ev_w);
        y += pitch;
    }
    if !d.days.is_empty() {
        sl.text("kicker_y", &d.week_label, Tx::l(k("left_w") + 40.0, ky).cy());
        let hot_dates: Vec<u8> = rows.iter().filter(|r| r.special).map(|r| r.date).collect();
        for (at, (dow, date)) in d.days.iter().enumerate() {
            let cy0 = k("cell_top") + at as f32 * (k("cell_h") + k("cell_gap"));
            let count = d.count_on(*date);
            let hot = hot_dates.contains(date);
            sl.push(R::new(k("cell_x"), cy0, k("cell_w"), k("cell_h")).fill(if hot { "red" } else { "white" }).rx(20.0).op(if hot { 1.0 } else { 0.12 }));
            let op = if count > 0 { 1.0 } else { 0.78 };
            sl.text("cell_n", &date.to_string(), Tx::l(k("cell_x") + 28.0, cy0 + k("cell_h") / 2.0).cy().op(op));
            sl.text(if hot { "dow_w" } else { "dow" }, dow, Tx::l(k("cell_x") + 28.0 + 84.0, cy0 + k("cell_h") / 2.0).cy().op(op));
            let dot_x = k("cell_x") + k("cell_w") - 28.0 - count as f32 * k("dot") - count.saturating_sub(1) as f32 * k("dot_gap");
            dots(sl, dot_x, cy0 + k("cell_h") / 2.0, count, k("dot"), k("dot_gap"));
        }
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    ground(sl, None, Some(k("band_h")));
    sl.text("title_w", &d.title, Tx::l(k("mx"), 140.0).maxw(k("strip_x") - k("mx") - 30.0));
    sl.text("sub_s", &d.month, Tx::l(k("mx"), 196.0));
    if d.highlighted() && d.days.is_empty() {
        // A highlight longer than a week is written as its range where the week's strip would stand.
        sl.text("title_w", &d.week_label, Tx::l(k("strip_x"), 140.0).maxw(W - k("strip_x") - k("mx")));
    }
    if !d.days.is_empty() && (d.highlighted() || d.covers) {
        sl.text("kicker_y", &d.week_label, Tx::l(k("strip_x"), 40.0).cy());
        for (at, (dow, date)) in d.days.iter().enumerate() {
            let x = k("strip_x") + at as f32 * (k("strip_w") + k("strip_gap"));
            sl.push(R::new(x, k("strip_y"), k("strip_w"), 150.0).fill("white").rx(20.0).op(0.12));
            let count = d.count_on(*date);
            let op = if count > 0 { 1.0 } else { 0.78 };
            sl.text("dow", dow, Tx::c(x + k("strip_w") / 2.0, k("strip_y") + 34.0).cy().op(op));
            sl.text("cell_n", &date.to_string(), Tx::c(x + k("strip_w") / 2.0, k("strip_y") + 86.0).cy().op(op));
            dots(sl, x + k("strip_w") / 2.0 - (count as f32 * k("dot") + count.saturating_sub(1) as f32 * 8.0) / 2.0, k("strip_y") + 128.0, count, k("dot"), 8.0);
        }
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let sparse = n as f32 <= k("sparse_names");
    let cw = if sparse { k("sparse_w") } else { W - 2.0 * k("mx") };
    let colw = (cw - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = k("bottom") - k("top");
    let pitch = (avail / per as f32).min(if sparse { 130.0 } else { 999.0 });
    let rh = pitch - k("row_gap");
    let y0 = if sparse { k("top") + (avail - per as f32 * pitch) / 2.0 } else { k("top") };
    let day_w = if d.spans { names.iter().map(|x| sl.measure("bday_day_hl", &d.day_of(x), Some(40.0), None)).fold(k("day_w"), f32::max) + 8.0 } else { k("day_w") };
    let name_w = colw - k("card_pad") - day_w - k("name_gap") - k("name_pad");
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let hi = if sparse { k("sparse_size") } else { grown_size(k("size_max"), k("sparse_size"), avail, per, 1.7) };
    let size = largest_name_size(sl, &list, "name_hl", name_w, hi, k("size_floor")).ok_or(Split)?;
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = k("mx") + c as f32 * (colw + k("col_gap"));
        let cy = y0 + r as f32 * pitch + rh / 2.0;
        sl.push(R::new(cx, cy - rh / 2.0, colw, rh).fill(if nm.hl { "hl" } else { "white" }).rx(16.0));
        if nm.hl {
            sl.push(R::new(cx, cy - rh / 2.0, k("bar_w"), rh).fill("red").rx(4.0));
        }
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &d.day_of(nm), Tx::l(cx + k("card_pad") + k("bar_w") - 4.0, cy).cy().size(40.0_f32.min(size)));
        sl.text(role, &text, Tx::l(cx + k("card_pad") + day_w + k("name_gap"), cy).cy().size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (ky, base, sub_base, top, bottom) = (sl.g("kicker_y"), sl.g("title_base"), sl.g("sub_base"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    ground(sl, Some(k("left_w")), None);
    sl.text("kicker", &d.kicker(w::KAS), Tx::l(k("mx"), ky).cy().maxw(900.0));
    sl.text("title", &d.title, Tx::l(k("mx"), base).maxw(k("left_w") - 2.0 * k("mx")));
    if let Some(date) = &d.date_long {
        sl.text("sub", date, Tx::l(k("mx"), sub_base));
    }
    let avail = bottom - top;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let card_w = k("left_w") - k("mx") - 40.0;
    let face = sl.face_of("fund");
    let lines_at = |z: f32| -> (f32, f32, Vec<Vec<String>>) {
        let dw = funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
        let apw = rp_w + k("rp_gap") + dw;
        let lab_w = card_w - 2.0 * k("card_pad") - apw - 28.0;
        (apw, lab_w, funds.iter().map(|f| paragraph(sl, &f.label, &face, z, lab_w)).collect())
    };
    let size = largest(k("size_max"), k("size_floor"), |z| {
        let (_, _, lines) = lines_at(z);
        let maxl = lines.iter().map(Vec::len).max().unwrap_or(1);
        let rh = maxl as f32 * z * k("line_h") + 2.0 * k("pad_v");
        maxl <= 2 && n as f32 * (rh + k("row_gap")) <= avail
    })
    .ok_or(Split)?;
    let (apw, label_w, lines) = lines_at(size);
    let maxl = lines.iter().map(Vec::len).max().unwrap_or(1);
    let rh = maxl as f32 * size * k("line_h") + 2.0 * k("pad_v");
    for (i, f) in funds.iter().enumerate() {
        let y = top + i as f32 * (rh + k("row_gap"));
        let cy = y + rh / 2.0;
        sl.push(R::new(k("mx"), y, card_w, rh).fill("white").rx(18.0));
        let lh = size * k("line_h");
        for (at, line) in lines[i].iter().enumerate() {
            sl.text("fund", line, Tx::l(k("mx") + k("card_pad"), cy + (at as f32 - (lines[i].len() as f32 - 1.0) / 2.0) * lh).cy().size(size).maxw(label_w));
        }
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(k("mx") + card_w - k("card_pad"), cy).cy().size(size));
        sl.text("rp", "Rp", Tx::l(k("mx") + card_w - k("card_pad") - apw, cy).cy());
    }
    let (pw, ix) = (W - k("left_w"), k("left_w") + 52.0);
    let iw = pw - 104.0;
    let tv = fit::digits(d.total());
    let tz = largest(sl.ty_size("total"), 41.0, |z| sl.measure("total", &tv, Some(z), None) <= iw).unwrap_or(41.0);
    sl.text("total_label", &format!("{} {}", w::TOTAL, w::TOTAL_SUFFIX), Tx::l(ix, H / 2.0 - 120.0).cy().maxw(iw));
    sl.text("total_rp", "Rp", Tx::l(ix, H / 2.0 - 40.0).cy());
    sl.text("total", &tv, Tx::l(ix, H / 2.0 + 60.0).cy().size(tz).maxw(iw));
    sl.push(R::new(ix, H / 2.0 + 140.0, 140.0, 8.0).fill("yellow").rx(4.0));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (ky, base) = (sl.g("kicker_y"), sl.g("title_base"));
    let k = sl.sec("s04");
    ground(sl, Some(k("left_w")), None);
    let wide = k("left_w") - 2.0 * k("mx");
    let a = &d.account;
    sl.text("kicker", &d.kicker(w::BANK_KICKER), Tx::l(k("mx"), ky).cy().maxw(wide));
    sl.text("title", &d.title, Tx::l(k("mx"), base).size(68.0).maxw(wide));
    let nsz = largest_step(k("number_max"), k("number_floor"), 2.0, |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let note_size = sl.ty_size("note");
    let lines = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide - 2.0 * k("note_pad")) };
    let nlh = (note_size * 1.36).round();
    let note_h = if lines.is_empty() { 0.0 } else { lines.len() as f32 * nlh + 56.0 };
    let ncap = sl.cap(&sl.face_of("number"), nsz);
    let stack = 52.0 + 30.0 + ncap + 40.0 + 14.0 + 70.0 + 40.0 + note_h;
    let top = (base + 70.0).max((H - k("card")) / 2.0 - 30.0 + k("card") / 2.0 - stack / 2.0);
    sl.text("bank", &a.bank, Tx::l(k("mx"), top + 46.0).maxw(wide));
    let nb = top + 46.0 + 30.0 + ncap;
    sl.text("number", &a.number, Tx::l(k("mx"), nb).size(nsz).maxw(wide));
    let y = nb + 40.0;
    sl.text("holder_l", w::HOLDER_LABEL, Tx::l(k("mx"), y + 14.0).cy());
    let hsz = largest(sl.ty_size("holder"), 40.0, |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(40.0);
    sl.text("holder", &a.holder, Tx::l(k("mx"), y + 14.0 + 70.0).size(hsz).maxw(wide));
    let y = y + 14.0 + 70.0 + 40.0;
    if !lines.is_empty() {
        sl.push(R::new(k("mx"), y, wide, note_h).fill("yellow").rx(22.0));
        for (i, line) in lines.iter().enumerate() {
            sl.text("note", line, Tx::l(k("mx") + k("note_pad"), y + 28.0 + (i as f32 + 0.5) * nlh).cy().maxw(wide - 2.0 * k("note_pad")));
        }
    }
    if have.qris {
        let (px, py) = (k("left_w") + (W - k("left_w") - k("card")) / 2.0, (H - k("card")) / 2.0 - 30.0);
        sl.push(R::new(px, py, k("card"), k("card")).fill("white").rx(28.0));
        sl.image(Pic::Qris, px, py, k("card"), k("card"), 28.0, true, Pos::Mid);
        sl.text("caption", w::QRIS_CAPTION, Tx::c(px + k("card") / 2.0, py + k("card") + 64.0).maxw(k("card") + 60.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let ky = sl.g("kicker_y");
    let k = sl.sec("s05");
    ground(sl, Some(k("left_w")), None);
    sl.push(R::new(0.0, 0.0, k("left_w"), H).fill("light"));
    if have.photo {
        sl.image(Pic::Photo, k("left_w"), 0.0, W - k("left_w"), H, 0.0, false, Pos::Max);
        sl.push(R::new(k("left_w"), k("shade_y"), W - k("left_w"), H - k("shade_y")).fill("grad:shade"));
    }
    sl.text("kicker", &d.kicker(d.date_long.as_deref().unwrap_or("")), Tx::l(k("mx"), ky).cy().maxw(k("text_w")));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: k("mx"), top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    let px = k("left_w") + 56.0;
    sl.text("kicker_y", w::VERSE_KICKER, Tx::l(px, 904.0).cy());
    sl.text("ref", &d.reference, Tx::l(px, 1004.0).maxw(W - px - 40.0));
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let k = sl.sec("s06");
    ground(sl, None, None);
    sl.push(R::new(0.0, 0.0, k("left_w"), H).fill("light"));
    if have.photo {
        sl.image(Pic::Photo, k("left_w"), 0.0, W - k("left_w"), H, 0.0, false, Pos::Mid);
    } else {
        // The date is the picture: the day as a big numeral on the navy panel.
        sl.push(R::new(k("left_w"), 0.0, W - k("left_w"), H).fill("navy"));
        let cx = k("left_w") + (W - k("left_w")) / 2.0;
        if let Some((dow, day, month)) = &d.date_card {
            sl.text("dow_big", dow, Tx::c(cx, H / 2.0 - 220.0).cy());
            sl.text("day_big", day, Tx::c(cx, H / 2.0 + 10.0).cy());
            sl.text("dow_big", month, Tx::c(cx, H / 2.0 + 250.0).cy().fill("white"));
        }
        sl.push(R::new(cx - 70.0, H / 2.0 + 330.0, 140.0, 8.0).fill("yellow").rx(4.0));
    }
    let wide = k("text_w");
    let face = sl.face_of("title");
    let words = fit::words(&d.title);
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.36).round();
    let lead = d.paragraphs.first().cloned().unwrap_or_default();
    let rest = d.paragraphs.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    let lead_lines = paragraph(sl, &lead, &sl.face_of("n_body_b"), body_size, wide);
    let rest_lines = if rest.is_empty() { Vec::new() } else { paragraph(sl, &rest, &sl.face_of("n_body"), body_size, wide - 2.0 * k("note_pad")) };
    let box_h = if rest_lines.is_empty() { 0.0 } else { rest_lines.len() as f32 * blh + 56.0 };
    let body_h = lead_lines.len() as f32 * blh + if box_h > 0.0 { 28.0 + box_h } else { 0.0 };
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, 0.0);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3 && wrap_at(z).len() as f32 * z * k("title_lh") + 40.0 + body_h + 56.0 <= H - 120.0);
    let z = sl.must(z, k("title_floor"))?;
    let lines = wrap_at(z);
    let block = 32.0 + 36.0 + lines.len() as f32 * z * k("title_lh") + 44.0 + body_h;
    let top = (H - block) / 2.0;
    sl.text("kicker", w::NOTICE_KICKER, Tx::l(k("mx"), top + 16.0).cy());
    let mut y = top + 32.0 + 36.0;
    let cap = sl.cap(&face, z);
    for (i, line) in lines.iter().enumerate() {
        sl.text("title", line, Tx::l(k("mx"), y + (i as f32 + 0.5) * z * k("title_lh") + cap / 2.0).size(z).maxw(wide));
    }
    y += lines.len() as f32 * z * k("title_lh") + 44.0;
    for line in &lead_lines {
        sl.text("n_body_b", line, Tx::l(k("mx"), y + blh * 0.78).maxw(wide));
        y += blh;
    }
    if box_h > 0.0 {
        y += 28.0;
        sl.push(R::new(k("mx"), y, wide, box_h).fill("yellow").rx(22.0));
        for (i, line) in rest_lines.iter().enumerate() {
            sl.text("n_body", line, Tx::l(k("mx") + k("note_pad"), y + 28.0 + (i as f32 + 0.5) * blh).cy().maxw(wide - 2.0 * k("note_pad")));
        }
    }
    Ok(())
}
