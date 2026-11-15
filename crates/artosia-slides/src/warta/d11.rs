//! Direction 11, Liturgy: a vestment ground in a double gold frame, a small cross, parchment bands and Roman capitals over a book serif.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have};
use super::fit;
use super::{Fits, Item, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_name_size, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// Which part of a long card a slide is, centred just inside the frame.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::c(W / 2.0, sl.g("foot_cy")).cy());
}

fn gold_rule(sl: &mut Sheet, x: f32, y: f32, width: f32, height: f32, op: f32) {
    sl.push(R::new(x, y, width, height).fill("gold").op(op));
}

/// The ground, the double frame with its corner diamonds, the cross between two rules, the kicker and the title.
fn page(sl: &mut Sheet, kicker: &str, title: Option<&str>) {
    let (frame, gap, dia, pad) = (sl.g("frame"), sl.g("frame_gap"), sl.g("dia"), sl.g("pad"));
    let (w, h, cx) = (W, super::H, W / 2.0);
    sl.push(R::new(0.0, 0.0, w, h).fill("ground2"));
    sl.push(R::new(frame, frame, w - 2.0 * frame, h - 2.0 * frame).fill("ground").border(sl.g("frame_sw"), "gold"));
    let inner = frame + gap;
    sl.push(R::new(inner, inner, w - 2.0 * inner, h - 2.0 * inner).border(sl.g("frame_sw2"), "gold").bop(sl.g("frame_op2")));
    for (x, y) in [(inner, inner), (inner, h - inner), (w - inner, inner), (w - inner, h - inner)] {
        sl.push(R::new(x - dia / 2.0, y - dia / 2.0, dia, dia).fill("gold").rot(45.0));
    }
    let (cy, cw, ch, arm, arm_h, up) = (sl.g("cross_cy"), sl.g("cross_w"), sl.g("cross_h"), sl.g("cross_arm"), sl.g("cross_arm_h"), sl.g("cross_up"));
    gold_rule(sl, cx - cw / 2.0, cy - ch / 2.0, cw, ch, 1.0);
    gold_rule(sl, cx - arm / 2.0, cy - up - arm_h / 2.0, arm, arm_h, 1.0);
    let (rx, rgap, rw) = (sl.g("rule_x"), sl.g("rule_gap"), sl.g("rule_w"));
    gold_rule(sl, rx, cy - rw / 2.0, cx - rgap - rx, rw, 1.0);
    gold_rule(sl, cx + rgap, cy - rw / 2.0, cx - rgap - rx, rw, 1.0);
    sl.text("kicker", kicker, Tx::c(cx, sl.g("kicker_cy")).cy().maxw(w - 2.0 * pad));
    if let Some(title) = title.filter(|title| !title.is_empty()) {
        sl.text("title", title, Tx::c(cx, sl.g("title_cy")).cy().maxw(w - 2.0 * pad));
    }
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, bottom) = (sl.g("pad"), W - sl.g("pad"), sl.g("body_bot"));
    let k = sl.sec("s01");
    let (top, over, rx) = (k("top"), sl.g("band_over"), sl.g("band_rx"));
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, &d.church_kicker(""), Some(&d.title));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let breaks = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32;
    let avail = bottom - top;
    let (day_x, time_x, event_x, gap) = (k("day_x"), k("time_x"), k("event_x"), k("gap"));
    let ev_w = x2 - event_x - k("tag_room");
    let hi = if n as f32 <= k("sparse_rows") { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) + breaks * gap <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min((avail - breaks * gap) / n as f32);
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += gap;
            gold_rule(sl, mx, y - gap / 2.0 - 1.0, x2 - mx, 2.0, sl.g("rule_op"));
        }
        let cy = y + pitch / 2.0;
        let ink = |o: Tx| if r.special { o.fill("ink") } else { o };
        if r.special {
            sl.push(R::new(mx - over, y + 3.0, x2 - mx + 2.0 * over, pitch - 6.0).fill("cream").rx(rx));
        }
        if firsts[i] {
            sl.text("day", &r.day, ink(Tx::l(day_x, cy).cy().maxw(time_x - day_x - 20.0)));
        }
        sl.text("time", &r.time, ink(Tx::l(time_x, cy).cy().maxw(event_x - time_x - 20.0)));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if r.special {
            let tw = sl.measure("tag", d.w.special.as_str(), None, None) + 40.0;
            sl.push(R::new(x2 - tw, cy - k("tag_h") / 2.0, tw, k("tag_h")).fill("gold"));
            sl.text("tag", d.w.special.as_str(), Tx::l(x2 - tw + 20.0, cy).cy().fill("ink"));
        }
        y += pitch;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("pad"), W - sl.g("pad"), sl.g("body_top"), sl.g("body_bot"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, &d.church_kicker(""), Some(&d.title));
    let sub = sl.text("sub", &d.month, Tx::l(mx, sl.g("sub_cy")).cy());
    if d.highlighted() {
        let rw = sl.measure("tag", &d.week_label, None, None) + 2.0 * k("chip_pad");
        let rx = (x2 - rw).max(sub.x + sub.w + 30.0);
        let cy = sl.g("sub_cy");
        sl.push(R::new(rx, cy - k("chip_h") / 2.0, x2 - rx, k("chip_h")).pill().border(k("chip_sw"), "gold"));
        sl.text("tag", &d.week_label, Tx::l(rx + k("chip_pad"), cy).cy().maxw(x2 - rx - 2.0 * k("chip_pad")));
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("gutter") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let day_w = names.iter().map(|x| sl.measure("bday_day_hl", &d.day_of(x), None, None)).fold(k("day_w"), f32::max);
    let name_w = colw - k("row_pad") - day_w - k("day_gap");
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let hi = grown_size(k("size_floor"), k("sparse_size_max"), avail, per, k("ratio"));
    let size = largest_name_size(sl, &list, "name_hl", name_w, hi, k("size_floor")).ok_or(Split)?;
    let pitch = (avail / per as f32).min(1.9 * size);
    let at = |c: usize| mx + c as f32 * (colw + k("gutter"));
    for c in 1..cols {
        gold_rule(sl, at(c) - k("gutter") / 2.0 - k("vrule_w") / 2.0, top, k("vrule_w"), bottom - top, k("vrule_op"));
    }
    // The bands go first so the names sit on them.
    for c in 0..cols {
        let run: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).collect();
        if let Some(first) = run.first() {
            let (over, y) = (sl.g("band_over"), top + (*first - c * per) as f32 * pitch);
            sl.push(R::new(at(c) - over, y + 3.0, colw + 2.0 * over, run.len() as f32 * pitch - 6.0).fill("cream").rx(sl.g("band_rx")));
        }
    }
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cy = top + r as f32 * pitch + pitch / 2.0;
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &d.day_of(nm), Tx::l(at(c) + k("row_pad"), cy).cy());
        sl.text(role, &text, Tx::l(at(c) + k("row_pad") + day_w + k("day_gap"), cy).cy().size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2) = (sl.g("pad"), W - sl.g("pad"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, &d.church_kicker(""), Some(&d.title));
    if let Some(date) = d.date_long.as_deref() {
        sl.text("sub", date, Tx::c(W / 2.0, sl.g("sub_cy")).cy().maxw(x2 - mx));
    }
    let (top, avail) = (k("rows_top"), k("rows_bot") - k("rows_top"));
    let rp_w = sl.measure("rp", "Rp", None, None);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let digits_at = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z + 2.0), None)).fold(0.0, f32::max);
    let lab_at = |z: f32| x2 - mx - (rp_w + k("rp_gap") + digits_at(sl, z)) - k("label_gap");
    let hi = grown_size(k("size_max"), k("sparse_size_max"), avail, n, k("ratio"));
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, lab_at, hi, k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let label_w = lab_at(size);
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    for (i, f) in funds.iter().enumerate() {
        let cy = top + i as f32 * pitch + pitch / 2.0;
        put_lines(sl, "fund", &cells[i], mx, cy, size, label_w);
        let amount = sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, cy).cy().size(size + 2.0));
        sl.text("rp", "Rp", Tx::r(amount.x - k("rp_gap"), cy).cy());
        if i + 1 < funds.len() {
            gold_rule(sl, mx, top + (i + 1) as f32 * pitch - 1.0, x2 - mx, 2.0, sl.g("rule_op"));
        }
    }
    let (ty, th, pad) = (k("total_top"), k("total_h"), k("total_pad"));
    sl.push(R::new(mx, ty, x2 - mx, th).fill("cream").rx(sl.g("band_rx")));
    let cy = ty + th / 2.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(mx + pad, cy).cy());
    let rp = sl.measure("total", "Rp", Some(k("total_rp")), None) + k("rp_gap");
    let figure = sl.text("total", &fit::digits(d.total()), Tx::r(x2 - pad, cy).cy().maxw(x2 - pad - (mx + pad + label.w) - 40.0 - rp));
    sl.text("total", "Rp", Tx::r(figure.x - k("rp_gap"), cy).cy().size(k("total_rp")));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mid, k) = ((sl.g("body_top") + sl.g("body_bot")) / 2.0, sl.sec("s04"));
    page(sl, &d.church_kicker(""), Some(&d.title));
    let (wide, cx) = if have.qris { (k("w_qr"), k("col_x") + k("w_qr") / 2.0) } else { (k("w"), W / 2.0) };
    let a = &d.account;
    let nsz = largest(sl.ty_size("number"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let (bcap, ncap, hcap) = (sl.cap(&sl.face_of("bank"), sl.ty_size("bank")), sl.cap(&sl.face_of("number"), nsz), sl.cap(&sl.face_of("holder"), hsz));
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let (gap, rules) = (44.0, 3.0 + k("rule_gap") + 1.5);
    let stack = bcap + gap + ncap + gap + rules + nlh + hcap + 40.0 + if notes.is_empty() { 0.0 } else { 20.0 + notes.len() as f32 * nlh };
    let top = mid - stack / 2.0;
    sl.need(top >= sl.g("body_top") - 20.0)?;
    sl.text("bank", &a.bank, Tx::c(cx, top + bcap / 2.0).cy().maxw(wide));
    sl.text("number", &a.number, Tx::c(cx, top + bcap + gap + ncap / 2.0).cy().size(nsz).maxw(wide));
    let ry = top + bcap + gap + ncap + gap;
    gold_rule(sl, cx - wide / 2.0, ry, wide, 3.0, 1.0);
    gold_rule(sl, cx - wide / 2.0, ry + 3.0 + k("rule_gap"), wide, 1.5, 1.0);
    let ay = ry + rules + nlh / 2.0;
    sl.text("note", d.w.holder.as_str(), Tx::c(cx, ay).cy());
    let hy = ay + nlh / 2.0 + 20.0 + hcap / 2.0;
    sl.text("holder", &a.holder, Tx::c(cx, hy).cy().size(hsz).maxw(wide));
    let mut yy = hy + hcap / 2.0 + 20.0 + nlh / 2.0 + 20.0;
    for line in &notes {
        sl.text("note", line, Tx::c(cx, yy).cy().maxw(wide));
        yy += nlh;
    }
    if have.qris {
        // The code is dark on light, so it sits on a cream plate in a gold frame on every ground.
        let (plate, px, py) = (k("card"), k("qr_x"), mid - k("card") / 2.0 - 20.0);
        sl.push(R::new(px, py, plate, plate).fill("cream").border(k("plate_sw"), "gold"));
        sl.image(Pic::Qris, px + k("pad"), py + k("pad"), plate - 2.0 * k("pad"), plate - 2.0 * k("pad"), 0.0, true, Pos::Mid);
        sl.text("caption", d.w.scan.as_str(), Tx::c(px + plate / 2.0, py + plate + k("cap_gap")).cy().maxw(plate + 120.0));
    }
    Ok(())
}

/// A gold-framed picture at the right of the page.
fn framed(sl: &mut Sheet, k: &dyn Fn(&str) -> f32) {
    let (pw, sw) = (k("photo_w"), k("photo_sw"));
    let px = W - sl.g("pad") - pw;
    sl.push(R::new(px, k("photo_y"), pw, k("photo_h")).border(sw, "gold"));
    sl.image(Pic::Photo, px + sw, k("photo_y") + sw, pw - 2.0 * sw, k("photo_h") - 2.0 * sw, 0.0, false, Pos::Mid);
}

/// The text column of a slide with or without a picture: its left edge, width and centre.
fn column(sl: &Sheet, k: &dyn Fn(&str) -> f32, photo: bool) -> (f32, f32, f32) {
    let wide = if photo { k("w_pic") } else { k("w") };
    let cx = if photo { sl.g("pad") + wide / 2.0 } else { W / 2.0 };
    (cx - wide / 2.0, wide, cx)
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let k = sl.sec("s05");
    page(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")), None);
    let (x, wide, cx) = column(sl, &k, have.photo);
    let rcy = k("ref_cy");
    sl.text("ref", &d.reference, Tx::c(cx, rcy).cy().maxw(wide));
    gold_rule(sl, x, rcy - k("ref_rule"), wide, 2.0, 1.0);
    gold_rule(sl, x, rcy + k("ref_rule") - 2.0, wide, 2.0, 1.0);
    let (top, room, from) = (k("top"), k("bottom") - k("top"), sl.items.len());
    let face = sl.face_of("verse");
    let z = verse(sl, &d.verses, &VerseBox { x, top, w: wide, room, font: &face, section: "s05" })?;
    centre_lines(sl, from, cx, top, room, k("lh") * z);
    if have.photo {
        framed(sl, &k);
    }
    Ok(())
}

/// Moves the texts added from `from` so each line of height `line` is centred on `cx` and the block in `room` below `top`.
fn centre_lines(sl: &mut Sheet, from: usize, cx: f32, top: f32, room: f32, line: f32) {
    let which = |y: f32| ((y - top) / line).floor().max(0.0) as usize;
    let items = &mut sl.items[from..];
    let lines = items.iter().filter_map(|item| if let Item::Text(t) = item { Some(which(t.y)) } else { None }).max().map_or(0, |last| last + 1);
    let mut span = vec![(f32::MAX, f32::MIN); lines];
    for item in items.iter() {
        if let Item::Text(t) = item {
            let at = &mut span[which(t.y)];
            *at = (at.0.min(t.x), at.1.max(t.x + t.w));
        }
    }
    let down = (room - lines as f32 * line) / 2.0;
    for item in items.iter_mut() {
        if let Item::Text(t) = item {
            let (a, b) = span[which(t.y)];
            t.x += cx - (a + b) / 2.0;
            t.y += down;
        }
    }
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let k = sl.sec("s06");
    page(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")), None);
    let (_, wide, cx) = column(sl, &k, have.photo);
    if have.photo {
        framed(sl, &k);
    }
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= k("title_lines") as usize), k("title_floor"))?;
    let lines = wrap_at(z);
    let tl = z * k("title_lh");
    let (body_face, body_size) = (sl.face_of("n_body"), sl.ty_size("n_body"));
    let blh = (body_size * 1.4).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let nb: usize = body.iter().map(Vec::len).sum();
    let (rule, gap) = (3.0, 40.0);
    let block = lines.len() as f32 * tl + gap + rule + gap + nb as f32 * blh + body.len().saturating_sub(1) as f32 * k("line_gap");
    let top = (k("top") + k("bottom") - block) / 2.0;
    // A body taller than the room between the header and the frame does not fit at this size.
    sl.need(top >= k("top"))?;
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::c(cx, top + (i as f32 + 0.5) * tl).cy().size(z).maxw(wide));
    }
    let mut y = top + lines.len() as f32 * tl + gap;
    gold_rule(sl, cx - k("rule_w") / 2.0, y, k("rule_w"), rule, 1.0);
    y += rule + gap;
    for para in &body {
        for line in para {
            sl.text("n_body", line, Tx::c(cx, y + blh / 2.0).cy().maxw(wide));
            y += blh;
        }
        y += k("line_gap");
    }
    Ok(())
}
