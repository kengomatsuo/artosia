//! Direction 12, Glass: the church's photograph under one scrim, every row on one frosted plate and a script title.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have};
use super::fit;
use super::{Fits, H, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, largest, name_in, paragraph, put_lines, verse};

pub fn draw(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    match d.kind {
        NoticeKind::Schedule => s01(sl, d, have),
        NoticeKind::Birthdays => s02(sl, d, have),
        NoticeKind::Money => s03(sl, d, have),
        NoticeKind::Account => s04(sl, d, have),
        NoticeKind::Verse => s05(sl, d, have),
        NoticeKind::Plain => s06(sl, d, have),
    }
}

/// The part of a long card, white on the scrim under the plate.
pub fn foot(sl: &mut Sheet, note: &str) {
    let (x, y) = (sl.g("mx"), sl.g("foot_cy"));
    sl.text("foot", note, Tx::l(x, y).cy());
}

/// The ground (the photograph over the colourway's gradient), the scrim, the kicker, the script title and `sub` at the right.
/// `reserve` is the width at the right of the title that something else takes.
fn page(sl: &mut Sheet, d: &Data, have: Have, title: Option<&str>, sub: Option<&str>, reserve: f32) {
    let (mx, ky, sub_cy) = (sl.g("mx"), sl.g("kicker_cy"), sl.g("sub_cy"));
    let x2 = W - mx;
    sl.push(R::new(0.0, 0.0, W, H).fill("grad:ground"));
    if have.photo {
        sl.image(Pic::Photo, 0.0, 0.0, W, H, 0.0, false, Pos::Mid);
    }
    let scrim = sl.g("scrim_op");
    sl.push(R::new(0.0, 0.0, W, H).fill("scrim").op(scrim));
    let op = sl.g("kicker_op");
    let left = sl.text("kicker", &d.kicker(""), Tx::l(mx, ky).cy().op(op).maxw(x2 - mx - sl.mark_reserve()));
    sl.mark_after(left.x + left.w);
    if let Some(title) = title.filter(|title| !title.is_empty()) {
        let (x, base) = (mx - sl.g("title_dx"), sl.g("title_base"));
        sl.text("title", title, Tx::l(x, base).maxw(x2 - x - reserve));
    }
    if let Some(sub) = sub.filter(|sub| !sub.is_empty()) {
        sl.text("sub", sub, Tx::r(x2, sub_cy).cy().maxw(x2 - mx));
    }
}

/// The frosted plate: a flat translucent rounded rectangle.
fn plate(sl: &mut Sheet, x: f32, y: f32, w: f32, h: f32) {
    let (op, rx) = (sl.g("plate_op"), sl.g("plate_rx"));
    sl.push(R::new(x, y, w, h).fill("plate").op(op).rx(rx));
}

/// The highlight: a tint with an accent bar at its left edge, from `x` over `w`.
fn band(sl: &mut Sheet, x: f32, y: f32, w: f32, h: f32) {
    let (rx, bar) = (sl.g("band_rx"), sl.g("bar_w"));
    sl.push(R::new(x, y, w, h).fill("accent_tint").rx(rx));
    sl.push(R::new(x, y, bar, h).fill("accent").rx(bar / 2.0));
}

fn rule(sl: &mut Sheet, x: f32, y: f32, w: f32) {
    let (h, op) = (sl.g("rule_w"), sl.g("rule_op"));
    sl.push(R::new(x, y - h / 2.0, w, h).fill("mute").op(op));
}

fn s01(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (px, py, ph, pad) = (sl.g("plate_x"), sl.g("plate_y"), sl.g("plate_h"), sl.g("plate_pad"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, d, have, Some(&d.title), Some(&d.week_short), 0.0);
    let (left, right) = (px + pad + sl.g("day_in"), W - px - pad - sl.g("day_in"));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let gaps = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32 * k("group_gap");
    let day_w = (0..rows.len()).filter(|i| firsts[*i]).map(|i| sl.measure("day", &rows[i].day, None, None)).fold(0.0, f32::max) + 2.0 * k("pill_pad");
    let time_w = rows.iter().map(|r| sl.measure("time", &r.time, None, None)).fold(0.0, f32::max);
    let (time_x, special) = (left + day_w + k("gap_x"), rows.iter().any(|r| r.special));
    let event_x = time_x + time_w + k("gap_x");
    let tag_w = sl.measure("tag", &d.w.special, None, None) + 2.0 * k("tag_pad");
    let ev_w = right - event_x - if special { tag_w + 24.0 } else { 0.0 };
    let (top, bottom) = (k("top"), k("bottom"));
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) + gaps <= bottom - top).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min((bottom - top - gaps) / n as f32);
    // The plate holds the rows and no more, up to the whole height.
    let used = (2.0 * (top - py) + n as f32 * pitch + gaps).min(ph);
    plate(sl, px, py, W - 2.0 * px, used);
    let (band_over, pill_h) = (sl.g("band_over"), k("pill_h").min(pitch - 8.0));
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += k("group_gap");
        }
        let mid = y + pitch / 2.0;
        if r.special {
            band(sl, px + pad - band_over, y + 2.0, W - 2.0 * px - 2.0 * pad + 2.0 * band_over, pitch - 4.0);
        }
        if firsts[i] {
            sl.push(R::new(left, mid - pill_h / 2.0, day_w, pill_h).fill(if r.special { "plate" } else { "accent_tint" }).pill());
            sl.text("day", &r.day, Tx::c(left + day_w / 2.0, mid).cy().maxw(day_w - k("pill_pad")));
        }
        sl.text("time", &r.time, Tx::l(time_x, mid).cy().maxw(time_w + 4.0));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, mid, size, ev_w);
        if r.special {
            sl.push(R::new(right - tag_w, mid - k("tag_h") / 2.0, tag_w, k("tag_h")).fill("accent").pill());
            sl.text("tag", &d.w.special, Tx::c(right - tag_w / 2.0, mid).cy());
        }
        y += pitch;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (px, py, ph, pad, pv) = (sl.g("plate_x"), sl.g("plate_y"), sl.g("plate_h"), sl.g("plate_pad"), sl.g("plate_pad_v"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    let (full, x2) = (k("per") as usize, W - sl.g("mx"));
    sl.need(n <= full * k("cols") as usize)?;
    // Three columns share the names; the plate holds the rows of the longest and no more.
    let per = n.div_ceil(k("cols") as usize).max(1);
    // The range sits in a white pill under the month, only when a name of the page is highlighted.
    let pill = d.highlighted().then(|| (d.week_label.as_str(), sl.measure("tag", &d.week_label, None, None) + 2.0 * sl.g("pill_pad")));
    page(sl, d, have, Some(&d.title), Some(&d.month), pill.map_or(0.0, |(_, w)| w + 40.0));
    if let Some((label, w)) = pill {
        let (h, cy, op) = (sl.g("pill_h"), sl.g("pill_cy"), sl.g("plate_op"));
        sl.push(R::new(x2 - w, cy - h / 2.0, w, h).fill("plate").op(op).pill());
        sl.text("tag", label, Tx::r(x2 - sl.g("pill_pad"), cy).cy().fill("accent"));
    }
    let pitch = (ph - 2.0 * pv) / full as f32;
    plate(sl, px, py, W - 2.0 * px, 2.0 * pv + per as f32 * pitch);
    let gutter = k("gutter");
    let cw = (W - 2.0 * px - 2.0 * pad - 2.0 * gutter) / 3.0;
    let day_w = names.iter().map(|x| sl.measure("bday_day", &d.day_of(x), None, None)).fold(sl.g("day_w"), f32::max);
    let (day_in, nx) = (sl.g("day_in"), sl.g("day_in") + day_w + sl.g("day_gap"));
    let name_w = cw - nx;
    let role_of = |hl: bool| if hl { "name_hl" } else { "name" };
    // The largest size that shortens no more names than the floor size does.
    let cuts_at = |z: f32| -> Option<usize> {
        names.iter().try_fold(0, |cuts, nm| fit::ladder(&nm.name, name_w, &|s| sl.measure(role_of(nm.hl), s, Some(z), None)).map(|text| cuts + usize::from(text != nm.name)))
    };
    let floor = k("size_floor");
    let base = cuts_at(floor);
    sl.need(base.is_some())?;
    let size = largest(sl.ty_size("name_hl"), floor, |z| cuts_at(z).zip(base).is_none_or(|(cuts, base)| cuts <= base)).unwrap_or(floor);
    let y0 = py + pv;
    let x_of = |c: usize| px + pad + c as f32 * (cw + gutter);
    let over = sl.g("band_over");
    for c in 0..n.div_ceil(per) {
        let mut r = 0;
        while r < per {
            if !names.get(c * per + r).is_some_and(|nm| nm.hl) {
                r += 1;
                continue;
            }
            let from = r;
            while names.get(c * per + r).is_some_and(|nm| nm.hl) && r < per {
                r += 1;
            }
            band(sl, x_of(c) - over, y0 + from as f32 * pitch + 3.0, cw + 2.0 * over, (r - from) as f32 * pitch - 6.0);
        }
    }
    for (i, nm) in names.iter().enumerate() {
        let (x, cy) = (x_of(i / per), y0 + ((i % per) as f32 + 0.5) * pitch);
        let role = role_of(nm.hl);
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text("bday_day", &d.day_of(nm), Tx::l(x + day_in, cy).cy().maxw(day_w));
        sl.text(role, &text, Tx::l(x + nx, cy).cy().size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (px, py, ph, pad, pv) = (sl.g("plate_x"), sl.g("plate_y"), sl.g("plate_h"), sl.g("plate_pad"), sl.g("plate_pad_v"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, have, Some(&d.title), d.date_long.as_deref(), 0.0);
    plate(sl, px, py, k("plate_w"), ph);
    let (xl, xr) = (px + pad, px + k("plate_w") - pad);
    let pitch = (ph - 2.0 * pv) / n as f32;
    let digits_w = funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), None, None)).fold(0.0, f32::max);
    let col = digits_w + k("rp_gap") + sl.measure("rp", "Rp", None, None);
    let label_w = xr - xl - col - k("lab_gap");
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, _) = fit_cells(sl, "fund", &labels, |_| label_w, k("size_max"), k("size_floor"), |z, e| z + e <= pitch - 12.0).ok_or(Split)?;
    for (i, f) in funds.iter().enumerate() {
        let cy = py + pv + (i as f32 + 0.5) * pitch;
        put_lines(sl, "fund", &cells[i], xl, cy, size, label_w);
        let at = sl.text("amount", &fit::digits(f.rupiah), Tx::r(xr, cy).cy());
        sl.text("rp", "Rp", Tx::r(at.x - k("rp_gap"), cy).cy());
        if i + 1 < funds.len() {
            rule(sl, xl, cy + pitch / 2.0, xr - xl);
        }
    }
    // The total is a solid block over the photograph, never bare photograph.
    let (tx, ty, th, tpad, mx) = (sl.g("tot_x"), sl.g("tot_y"), sl.g("tot_h"), sl.g("tot_pad"), sl.g("mx"));
    let tw = W - mx - tx;
    let rx = sl.g("plate_rx");
    sl.push(R::new(tx, ty, tw, th).fill("accent").rx(rx));
    sl.text("total_label", &d.w.total_gifts, Tx::l(tx + tpad, ty + sl.g("tot_label_dy")).cy().maxw(tw - 2.0 * tpad));
    sl.text("total_rp", "Rp", Tx::l(tx + tpad, ty + sl.g("tot_rp_dy")).cy());
    let total = fit::digits(d.total());
    let fits = |z: f32| sl.measure("total", &total, Some(z), None) <= tw - 2.0 * tpad;
    let z = sl.must(largest(sl.g("total_fit"), sl.g("total_min"), fits), sl.g("total_min"))?;
    sl.text("total", &total, Tx::l(tx + tpad, ty + sl.g("tot_fig_dy")).cy().size(z));
    Ok(())
}

/// A number in two lines: its groups cut in half, or its digits when it has no groups.
fn two_lines(number: &str) -> Vec<String> {
    let groups: Vec<&str> = number.split_whitespace().collect();
    if groups.len() >= 2 {
        let half = groups.len().div_ceil(2);
        return vec![groups[..half].join(" "), groups[half..].join(" ")];
    }
    let chars: Vec<char> = number.chars().collect();
    let half = chars.len().div_ceil(2);
    vec![chars[..half].iter().collect(), chars[half..].iter().collect()]
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (py, ph) = (sl.g("plate_y"), sl.g("plate_h"));
    let k = sl.sec("s04");
    page(sl, d, have, Some(&d.title), Some(&d.w.account), 0.0);
    let wp = if have.qris { k("w_qr") } else { k("w") };
    let px = if have.qris { sl.g("plate_x") } else { (W - wp) / 2.0 };
    plate(sl, px, py, wp, ph);
    let (x, wide) = (px + k("pad"), wp - 2.0 * k("pad"));
    let a = &d.account;
    let (lines, nsz) = match largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide) {
        Some(z) => (vec![a.number.clone()], z),
        None => {
            let two = two_lines(&a.number);
            let fits = |z: f32| two.iter().all(|line| sl.measure("number", line, Some(z), None) <= wide);
            let z = sl.must(largest(k("number_max"), k("number_floor2"), fits), k("number_floor2"))?;
            (two, z)
        }
    };
    let hsz = sl.must(largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide), k("holder_floor"))?;
    let (face, size) = (sl.face_of("note"), sl.ty_size("note"));
    let notes = if a.note.is_empty() { Vec::new() } else { super::paragraphs(sl, &a.note, &face, size, wide) };
    let (nlh, npitch) = ((size * 1.35).round(), nsz * 1.1);
    // Offsets from the top of the stack, which is then centred on the plate.
    let bank = sl.cap(&sl.face_of("bank"), sl.ty_size("bank"));
    let first = bank + 40.0 + sl.cap(&sl.face_of("number"), nsz);
    let last = first + (lines.len() as f32 - 1.0) * npitch;
    let label = last + 72.0;
    let holder = label + 24.0 + sl.cap(&sl.face_of("holder"), hsz);
    let end = if notes.is_empty() { holder } else { holder + 54.0 + nlh * (notes.len() as f32 - 0.2) } + 12.0;
    sl.need(end <= ph - 2.0 * sl.g("plate_pad_v"))?;
    let top = py + (ph - end) / 2.0;
    sl.text("bank", &a.bank, Tx::l(x, top + bank).maxw(wide));
    for (i, line) in lines.iter().enumerate() {
        sl.text("number", line, Tx::l(x, top + first + i as f32 * npitch).size(nsz).maxw(wide));
    }
    sl.text("caption", &d.w.holder, Tx::l(x, top + label));
    sl.text("holder", &a.holder, Tx::l(x, top + holder).size(hsz).maxw(wide));
    for (i, line) in notes.iter().enumerate() {
        sl.text("note", line, Tx::l(x, top + holder + 54.0 + nlh * (i as f32 + 0.8)).maxw(wide));
    }
    if have.qris {
        // The code is black on white, so it keeps its own white plate over the photograph.
        let (card, pad) = (k("card"), k("card_pad"));
        let (qx, qy) = (W - sl.g("plate_x") - card, py + (ph - card - k("qr_gap") - 22.0) / 2.0);
        sl.push(R::new(qx, qy, card, card).fill("white").rx(sl.g("band_rx")));
        sl.image(Pic::Qris, qx + pad, qy + pad, card - 2.0 * pad, card - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(qx + card / 2.0, qy + card + k("qr_gap") + 22.0).fill("white").maxw(card + 120.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (px, py, ph) = (sl.g("plate_x"), sl.g("plate_y"), sl.g("plate_h"));
    let k = sl.sec("s05");
    page(sl, d, have, Some(&d.w.verse), d.date_long.as_deref(), 0.0);
    // With the photograph the plate leaves its right side showing.
    let wp = if have.photo { k("w_pic") } else { k("w") };
    plate(sl, px, py, wp, ph);
    let (x, wide) = (px + k("pad"), wp - 2.0 * k("pad"));
    sl.text("ref", &d.reference, Tx::l(x, k("ref_base")).maxw(wide));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x, top: k("top"), w: wide, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (px, py, ph, mx) = (sl.g("plate_x"), sl.g("plate_y"), sl.g("plate_h"), sl.g("mx"));
    let k = sl.sec("s06");
    page(sl, d, have, None, d.date_long.as_deref(), 0.0);
    let (face, ls) = (sl.face_of("n_title"), sl.ty("n_title").ls);
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, k("w"), space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let wide = k("w") - 2.0 * k("pad");
    let (body_face, body_size) = (sl.face_of("n_body"), sl.ty_size("n_body"));
    let blh = (body_size * 1.4).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let body_h = body.iter().map(Vec::len).sum::<usize>() as f32 * blh + body.len().saturating_sub(1) as f32 * k("line_gap");
    let plate_h = if body.is_empty() { 0.0 } else { body_h + 2.0 * k("plate_pad") };
    let bottom_of = |z: f32| k("top") + sl.cap(&face, z) + (wrap_at(z).len().max(1) - 1) as f32 * k("title_lh") * z + 0.3 * z;
    let holds = |z: f32| wrap_at(z).len() <= k("title_lines") as usize && bottom_of(z) + k("plate_gap") + plate_h <= py + ph;
    let z = sl.must(largest(k("title_max"), k("title_floor"), holds), k("title_floor"))?;
    let (cap, lines, y) = (sl.cap(&face, z), wrap_at(z), bottom_of(z) + k("plate_gap"));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("top") + cap + i as f32 * k("title_lh") * z).size(z).maxw(k("w")));
    }
    if body.is_empty() {
        return Ok(());
    }
    sl.need(y + plate_h <= py + ph)?;
    plate(sl, px, y, k("w"), plate_h);
    let mut at = y + k("plate_pad");
    for para in &body {
        for line in para {
            sl.text("n_body", line, Tx::l(mx, at + blh * 0.8).maxw(wide));
            at += blh;
        }
        at += k("line_gap");
    }
    Ok(())
}
