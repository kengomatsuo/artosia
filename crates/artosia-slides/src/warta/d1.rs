//! Direction 1, Editorial greige: a hairline frame, didone caps over a grotesque, terracotta as the one accent.

use artosia_core::deck::NoticeKind;

use super::data::{self as w, Data, Have};
use super::fit;
use super::{Fits, H, fit_cells, put_lines, Pic, Pos, R, Sheet, Split, Tx, W, largest, largest_name_size, name_in, paragraph, paragraphs, verse, VerseBox};

const ROMAN: [&str; 10] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

pub fn draw(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    match d.kind {
        NoticeKind::Schedule => s01(sl, d, have),
        NoticeKind::Birthdays => s02(sl, d, have),
        NoticeKind::Money => s03(sl, d),
        NoticeKind::Account => s04(sl, d, have),
        NoticeKind::Verse => s05(sl, d, have),
        NoticeKind::Plain => s06(sl, d, have),
    }
}

fn page(sl: &mut Sheet, kicker: &str, kicker_r: Option<&str>) {
    let (frame, fw, mx, x2, ky) = (sl.g("frame"), sl.g("frame_w"), sl.g("mx"), sl.g("x2"), sl.g("kicker_y"));
    sl.push(R::new(0.0, 0.0, W, H).fill("paper"));
    sl.push(R::new(frame, frame, W - 2.0 * frame, H - 2.0 * frame).border(fw, "ink"));
    sl.text("kicker", kicker, Tx::l(mx, ky).cy().maxw(1180.0));
    if let Some(right) = kicker_r {
        sl.text("kicker", right, Tx::r(x2, ky).cy().maxw(700.0));
    }
}

fn rule(sl: &mut Sheet, x: f32, y: f32, width: f32, height: Option<f32>, op: Option<f32>) {
    let (fw, rop) = (sl.g("frame_w"), number(sl, "rule_op"));
    sl.push(R::new(x, y, width, height.unwrap_or(fw)).fill("ink").op(op.unwrap_or(rop)));
}

fn number(sl: &Sheet, key: &str) -> f32 {
    sl.d[key].as_f64().unwrap_or(0.0) as f32
}

fn masthead(sl: &mut Sheet, title: &str, sub: Option<&str>) -> super::Tm {
    let (mx, x2, base, ry, rw, rgap, fw) = (sl.g("mx"), sl.g("x2"), sl.g("title_base"), sl.g("rule_y"), sl.g("rule_w"), sl.g("rule_gap"), sl.g("frame_w"));
    let t = sl.text("title", title, Tx::l(mx, base).maxw(x2 - mx));
    if let Some(sub) = sub {
        sl.text("subtitle", sub, Tx::r(x2, base).maxw((x2 - mx - t.w - 40.0).max(200.0)));
    }
    sl.push(R::new(mx, ry, x2 - mx, rw).fill("ink"));
    sl.push(R::new(mx, ry + rw + rgap, x2 - mx, fw).fill("ink"));
    t
}

fn s01(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, &d.church_kicker(""), Some(w::WIB));
    masthead(sl, &d.title, (!d.week_short.is_empty()).then_some(d.week_short.as_str()));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let breaks = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32;
    let avail = bottom - top;
    let (event_x, tag_room, day_x, date_x, time_x, group_gap, ratio, ratio_max, band_pad) = (k("event_x"), k("tag_room"), k("day_x"), k("date_x"), k("time_x"), k("group_gap"), k("ratio"), k("ratio_max"), sl.g("band_pad"));
    let ev_w = x2 - event_x - tag_room;
    let sparse = n as f32 <= k("sparse_rows");
    let hi = if sparse { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| n as f32 * (ratio * z + e) + breaks * group_gap <= avail).ok_or(Split)?;
    let pitch = (ratio_max * size + extra).min((avail - breaks * group_gap) / n as f32);
    let cap = sl.cap(&sl.face_of("event"), size);
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += group_gap;
            rule(sl, mx, y - group_gap / 2.0, x2 - mx, None, None);
        }
        let (cy, special) = (y + pitch / 2.0, r.special);
        let by = cy + cap / 2.0;
        if special {
            let bx = mx - band_pad;
            sl.push(R::new(bx, y + 4.0, x2 - mx + 2.0 * band_pad, pitch - 8.0).fill("tint"));
            sl.push(R::new(bx, y + 4.0, 8.0, pitch - 8.0).fill("accent"));
        }
        if firsts[i] {
            sl.text("day_name", &r.day, Tx::l(day_x, by).maxw(date_x - day_x - 14.0));
            if r.date > 0 {
                sl.text("date_small", &format!("{} {}", r.date, r.month.chars().take(3).collect::<String>()), Tx::l(date_x, by).maxw(time_x - 20.0 - date_x));
            }
        }
        sl.text("time", &r.time, Tx::l(time_x, by).maxw(event_x - time_x - 20.0));
        put_lines(sl, if special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if special {
            let tw = sl.measure("tag", w::SPECIAL, None, None) + 40.0;
            let tx = x2 - tw;
            sl.push(R::new(tx, cy - k("tag_h") / 2.0, tw, k("tag_h")).border(2.0, "accent"));
            sl.text("tag", w::SPECIAL, Tx::l(tx + 20.0, cy).cy());
        }
        y += pitch;
    }
    // Little data: a picture takes the room the rows leave.
    let left = bottom - y;
    if sparse && have.photo && left >= k("photo_min") + k("photo_gap") + 26.0 {
        let ph = left - k("photo_gap") - 26.0;
        let py = y + k("photo_gap");
        sl.push(R::new(mx + 26.0, py + 26.0, x2 - mx - 26.0, ph).border(sl.g("frame_w"), "ink"));
        sl.image(Pic::Photo, mx, py, x2 - mx - 26.0, ph, 0.0, false, Pos::Mid);
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, &d.kicker(&d.month), None);
    let mh = masthead(sl, &d.title, None);
    let sub = sl.text("subtitle", &d.month, Tx::l(mh.x + mh.w + 34.0, base));
    if let Some((from, to)) = d.week {
        let week = format!("{}{}{}\u{2013}{}", w::THIS_WEEK, w::SEP, from, to);
        let rw = sl.measure("ribbon", &week, None, None) + 48.0;
        let rx = (x2 - rw).max(sub.x + sub.w + 30.0);
        let cy = base - 0.7 * 96.0 / 2.0;
        sl.push(R::new(rx, cy - 32.0, rw, 64.0).fill("accent"));
        sl.text("ribbon", &week, Tx::l(rx + 24.0, cy).cy());
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let sparse = n as f32 <= k("sparse_names") && have.photo;
    let cw = (x2 - mx) - if sparse { k("photo_w") + 60.0 } else { 0.0 };
    let colw = (cw - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let ratio = if sparse { k("sparse_ratio") } else { k("ratio") };
    let name_w = colw - k("day_w") - k("name_gap") - k("name_pad");
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let hi = (if sparse { k("sparse_size_max") } else { k("size_max") }).min((avail / per as f32 / ratio).floor());
    let size = largest_name_size(sl, &list, "name_hl", name_w, hi, k("size_floor")).ok_or(Split)?;
    let pitch = (avail / per as f32).min(1.9 * size);
    let y0 = if sparse { top + (avail - per as f32 * pitch) / 2.0 } else { top };
    // The week's bands go first so the names sit on them.
    for c in 0..cols {
        let idx: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).collect();
        if let Some(first) = idx.first() {
            let cx = mx + c as f32 * (colw + k("col_gap"));
            let at = y0 + (*first - c * per) as f32 * pitch;
            sl.push(R::new(cx - k("band_over"), at + 3.0, colw + 2.0 * k("band_over"), idx.len() as f32 * pitch - 6.0).fill("tint"));
            sl.push(R::new(cx - k("band_over"), at + 3.0, k("bar_w"), idx.len() as f32 * pitch - 6.0).fill("accent"));
        }
    }
    let face = sl.face_of("name");
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let cy = y0 + r as f32 * pitch + pitch / 2.0;
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        let by = cy + sl.cap(&face, size) / 2.0;
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &format!("{:02}", nm.day), Tx::r(cx + k("day_w"), by).size(48.0_f32.min(size + 4.0)));
        sl.text(role, &text, Tx::l(cx + k("day_w") + k("name_gap"), by).size(size).maxw(name_w));
        let last_in_col = r == per - 1 || i + 1 == n;
        if !last_in_col && !(nm.hl && names[i + 1].hl) {
            rule(sl, cx, cy + pitch / 2.0, colw, None, Some(0.16));
        }
    }
    if sparse {
        let (px, ph) = (x2 - k("photo_w"), avail - 26.0);
        sl.push(R::new(px + 26.0, top + 26.0, k("photo_w") - 26.0, ph).border(sl.g("frame_w"), "ink"));
        sl.image(Pic::Photo, px, top, k("photo_w") - 26.0, ph, 0.0, false, Pos::Mid);
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, &d.church_kicker(""), Some(w::KAS));
    masthead(sl, &d.title, d.date_long.as_deref());
    let total = d.total();
    let avail = bottom - top - k("total_h") - k("total_gap");
    let rp_w = sl.measure("rp", "Rp", None, None);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let lab_at = |z: f32| {
        let dw = funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
        x2 - mx - k("no_w") - (rp_w + k("rp_gap") + dw) - 40.0
    };
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, lab_at, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let dw = funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(size), None)).fold(0.0, f32::max);
    let amount_col = rp_w + k("rp_gap") + dw;
    let label_w = x2 - mx - k("no_w") - amount_col - 40.0;
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let cap = sl.cap(&sl.face_of("fund"), size);
    for (i, f) in funds.iter().enumerate() {
        let by = top + i as f32 * pitch + pitch / 2.0 + cap / 2.0;
        sl.text("fund_no", ROMAN.get(i).copied().unwrap_or(""), Tx::l(mx, by).maxw(k("no_w") - 10.0));
        put_lines(sl, "fund", &cells[i], mx + k("no_w"), by - cap / 2.0, size, label_w);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, by).size(size));
        sl.text("rp", "Rp", Tx::l(x2 - amount_col, by));
        if i + 1 < funds.len() {
            rule(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx, None, Some(0.18));
        }
    }
    let ty = bottom - k("total_h");
    rule(sl, mx, ty, x2 - mx, Some(3.0), Some(1.0));
    rule(sl, mx, ty + 11.0, x2 - mx, None, Some(1.0));
    let tb = ty + 28.0 + sl.cap(&sl.face_of("total"), sl.ty_size("total")) + 12.0;
    sl.text("total_label", &format!("{} {}", w::TOTAL, w::TOTAL_SUFFIX), Tx::l(mx, tb));
    sl.text("total", &fit::rupiah(total), Tx::r(x2, tb).maxw(x2 - mx - 420.0));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s04");
    page(sl, &d.church_kicker(""), Some(&d.title));
    let card = if have.qris { k("card") } else { 0.0 };
    let x0 = if have.qris { mx + card + k("frame_off") + k("col_gap") } else { mx };
    let wide = x2 - x0;
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let ncap = sl.cap(&sl.face_of("number"), nsz);
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let stack = 16.0 + 76.0 + 44.0 + ncap + 44.0 + 60.0 + 70.0 + 40.0 + 10.0 + notes.len() as f32 * nlh;
    let top = (H - stack) / 2.0;
    sl.text("kicker", w::BANK_KICKER, Tx::l(x0, top + 16.0).cy().fill("accent"));
    sl.text("bank", &a.bank, Tx::l(x0, top + 92.0).maxw(wide));
    let nb = top + 92.0 + 44.0 + ncap;
    sl.text("number", &a.number, Tx::l(x0, nb).size(nsz).maxw(wide));
    rule(sl, x0, nb + 44.0, wide, Some(3.0), Some(1.0));
    sl.text("atas", w::HOLDER_LABEL, Tx::l(x0, nb + 44.0 + 62.0));
    let hb = nb + 44.0 + 62.0 + 70.0;
    sl.text("holder", &a.holder, Tx::l(x0, hb).size(hsz).maxw(wide));
    rule(sl, x0, hb + 40.0, wide, None, Some(0.4));
    let mut yy = hb + 40.0 + 10.0 + nlh * 0.85;
    for line in &notes {
        sl.text("note", line, Tx::l(x0, yy).maxw(wide));
        yy += nlh;
    }
    if have.qris {
        let cy = (H - card) / 2.0 - 20.0;
        sl.push(R::new(mx + k("frame_off"), cy + k("frame_off"), card, card).border(sl.g("frame_w"), "ink"));
        sl.image(Pic::Qris, mx, cy, card, card, 0.0, true, Pos::Mid);
        sl.text("caption", w::QRIS_CAPTION, Tx::c(mx + k("frame_off") + card / 2.0, cy + card + k("frame_off") + 62.0).maxw(card));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")), None);
    let text_w = if have.photo { k("text_w") } else { sl.g("x2") - mx };
    sl.text("kicker", w::VERSE_KICKER, Tx::l(mx, k("kick_y")).cy().fill("accent"));
    sl.text("ref", &d.reference, Tx::l(mx, k("kick_y") + 104.0).maxw(text_w));
    sl.push(R::new(mx, k("kick_y") + 104.0 + 30.0, 96.0, 4.0).fill("accent"));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: text_w, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    if have.photo {
        let (px, py, pw, ph) = (k("photo_x"), k("photo_y"), k("photo_w"), k("photo_h"));
        sl.push(R::new(px + k("frame_off"), py + k("frame_off"), pw, ph).border(sl.g("frame_w"), "ink"));
        sl.image(Pic::Photo, px, py, pw, ph, 0.0, false, Pos::Max);
    }
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s06");
    page(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")), None);
    let (x0, wide, centre) = if have.photo {
        let stack = 2.0 * k("photo_h") + k("photo_gap");
        let py = (H - stack - k("frame_off")) / 2.0;
        sl.push(R::new(mx + k("frame_off"), py + k("frame_off"), k("photo_w"), stack).border(sl.g("frame_w"), "ink"));
        // Two photos stack; with one picture it is used twice, cropped from each end.
        let (first, second) = if have.photo2 { (Pos::Mid, Pos::Mid) } else { (Pos::Min, Pos::Max) };
        sl.image(Pic::Photo, mx, py, k("photo_w"), k("photo_h"), 0.0, false, first);
        sl.image(Pic::Photo2, mx, py + k("photo_h") + k("photo_gap"), k("photo_w"), k("photo_h"), 0.0, false, second);
        (k("text_x"), x2 - k("text_x"), false)
    } else {
        let (rw, rgap, fw) = (sl.g("rule_w"), sl.g("rule_gap"), sl.g("frame_w"));
        sl.push(R::new(mx, k("plate_top"), x2 - mx, rw).fill("ink"));
        sl.push(R::new(mx, k("plate_top") + rw + rgap, x2 - mx, fw).fill("ink"));
        sl.push(R::new(mx, k("plate_bottom"), x2 - mx, fw).fill("ink"));
        sl.push(R::new(mx, k("plate_bottom") + rgap + fw, x2 - mx, rw).fill("ink"));
        (W / 2.0, k("nopic_w"), true)
    };
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = largest_step(k("title_max"), k("title_floor"), 4.0, |z| wrap_at(z).len() <= 2).unwrap_or(k("title_floor"));
    let lines = wrap_at(z);
    let tl = z * k("title_lh");
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.4).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let nb: usize = body.iter().map(Vec::len).sum();
    let block = 16.0 + 44.0 + lines.len() as f32 * tl + 46.0 + 4.0 + 40.0 + nb as f32 * blh + body.len().saturating_sub(1) as f32 * k("line_gap");
    let top = (H - block) / 2.0;
    let place = |x: f32, y: f32| if centre { Tx::c(x, y) } else { Tx::l(x, y) };
    sl.text("kicker", w::NOTICE_KICKER, place(x0, top + 16.0).cy().fill("accent"));
    let mut y = top + 16.0 + 44.0;
    let cap = sl.cap(&face, z);
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, place(x0, y + (i as f32 + 0.5) * tl + cap / 2.0).size(z).maxw(wide));
    }
    y += lines.len() as f32 * tl + 46.0;
    sl.push(R::new(x0 - if centre { 48.0 } else { 0.0 }, y, 96.0, 4.0).fill("accent"));
    y += 4.0 + 40.0;
    for (pi, para) in body.iter().enumerate() {
        for line in para {
            let mut o = place(x0, y + blh * 0.8).maxw(wide);
            if pi == 0 {
                o = o.font("sans_bb");
            }
            sl.text("n_body", line, o);
            y += blh;
        }
        y += k("line_gap");
    }
    Ok(())
}

/// The largest size from `hi` down to `lo` in steps of `step` at which `ok` holds.
pub(super) fn largest_step(hi: f32, lo: f32, step: f32, ok: impl Fn(f32) -> bool) -> Option<f32> {
    let mut z = hi;
    while z >= lo {
        if ok(z) {
            return Some(z);
        }
        z -= step;
    }
    None
}
