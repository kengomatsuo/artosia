//! Direction 5, Simple: a plain ground, one sans and rows on hairlines; the light and the dark edition share these layouts.

use artosia_core::deck::NoticeKind;

use super::data::{self as w, Data, Have};
use super::fit;
use super::{Fits, H, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_name_size, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The ground, the church's name at the left and `right` (words, colour) at the right of the top line.
fn page(sl: &mut Sheet, d: &Data, right: Option<(&str, &str)>) {
    let (mx, x2, ky) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_y"));
    sl.push(R::new(0.0, 0.0, W, H).fill("paper"));
    let mut taken = 0.0;
    if let Some((words, colour)) = right.filter(|(words, _)| !words.is_empty()) {
        taken = sl.text("ribbon", words, Tx::r(x2, ky).cy().fill(colour).maxw((x2 - mx) / 2.0)).w + 40.0;
    }
    if !d.church.is_empty() {
        sl.text("kicker", &d.church, Tx::l(mx, ky).cy().maxw(x2 - mx - taken));
    }
}

fn masthead(sl: &mut Sheet, title: &str, sub: Option<&str>) {
    let (mx, x2, base) = (sl.g("mx"), sl.g("x2"), sl.g("title_base"));
    let t = sl.text("title", title, Tx::l(mx, base).maxw(x2 - mx));
    let at = t.x + t.w + 34.0;
    if let Some(sub) = sub.filter(|sub| at + sl.measure("subtitle", sub, Some(32.0), None) <= x2) {
        sl.text("subtitle", sub, Tx::l(at, base).maxw(x2 - at));
    }
}

fn rule(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let height = sl.g("rule_w");
    sl.push(R::new(x, y - height / 2.0, width, height).fill("rule"));
}

/// The highlight: a ground with a bar at its left.
fn band(sl: &mut Sheet, x: f32, y: f32, width: f32, height: f32) {
    let bar = sl.g("bar_w");
    sl.push(R::new(x, y, width, height).fill("tint"));
    sl.push(R::new(x, y, bar, height).fill("accent"));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, d, Some((d.week_short.as_str(), "mute")));
    masthead(sl, &d.title, None);
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let avail = bottom - top;
    let (event_x, day_x, date_x, time_x, ratio) = (k("event_x"), k("day_x"), k("date_x"), k("time_x"), k("ratio"));
    let ev_w = x2 - event_x;
    let hi = if n as f32 <= k("sparse_rows") { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| n as f32 * (ratio * z + e) <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let cap = sl.cap(&sl.face_of("event"), size);
    let band_pad = sl.g("band_pad");
    for (i, r) in rows.iter().enumerate() {
        let y = top + i as f32 * pitch;
        let (cy, by) = (y + pitch / 2.0, y + pitch / 2.0 + cap / 2.0);
        if r.special {
            band(sl, mx - band_pad, y + 3.0, x2 - mx + 2.0 * band_pad, pitch - 6.0);
        }
        if firsts[i] {
            sl.text("day_name", &r.day, Tx::l(day_x, by).maxw(date_x - day_x - 14.0));
            if r.date > 0 {
                sl.text("date_small", &format!("{} {}", r.date, r.month.chars().take(3).collect::<String>()), Tx::l(date_x, by).maxw(time_x - 20.0 - date_x));
            }
        }
        sl.text("time", &r.time, Tx::l(time_x, by).maxw(event_x - time_x - 20.0));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if i + 1 < rows.len() && !r.special && !rows[i + 1].special {
            rule(sl, mx, y + pitch, x2 - mx);
        }
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, d, d.highlighted().then_some((d.week_label.as_str(), "accent")));
    masthead(sl, &d.title, Some(&d.month));
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let day_w = if d.spans { names.iter().map(|x| sl.measure("bday_day_hl", &d.day_of(x), Some(48.0), None)).fold(k("day_w"), f32::max) } else { k("day_w") };
    let name_w = colw - day_w - k("name_gap") - k("name_pad");
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let hi = grown_size(k("size_max"), k("sparse_size_max"), avail, per, k("ratio"));
    let size = largest_name_size(sl, &list, "name_hl", name_w, hi, k("size_floor")).ok_or(Split)?;
    let pitch = (avail / per as f32).min(1.9 * size);
    // The highlight goes first so the names sit on it.
    for c in 0..cols {
        let idx: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).collect();
        if let Some(first) = idx.first() {
            let at = top + (*first - c * per) as f32 * pitch;
            band(sl, mx + c as f32 * (colw + k("col_gap")) - k("band_over"), at + 3.0, colw + 2.0 * k("band_over"), idx.len() as f32 * pitch - 6.0);
        }
    }
    let face = sl.face_of("name");
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let cy = top + r as f32 * pitch + pitch / 2.0;
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        let by = cy + sl.cap(&face, size) / 2.0;
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &d.day_of(nm), Tx::r(cx + day_w, by).size(48.0_f32.min(size)));
        sl.text(role, &text, Tx::l(cx + day_w + k("name_gap"), by).size(size).maxw(name_w));
        let last_in_col = r == per - 1 || i + 1 == n;
        if !last_in_col && !(nm.hl && names[i + 1].hl) {
            rule(sl, cx, cy + pitch / 2.0, colw);
        }
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, d.date_long.as_deref().map(|date| (date, "mute")));
    masthead(sl, &d.title, None);
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
        if i + 1 < funds.len() {
            rule(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx);
        }
    }
    let ty = bottom - k("total_h");
    sl.push(R::new(mx, ty, x2 - mx, 3.0).fill("ink"));
    let tb = ty + k("total_h") / 2.0 + sl.cap(&sl.face_of("total"), sl.ty_size("total")) / 2.0;
    sl.text("total_label", &d.w.total_gifts, Tx::l(mx, tb));
    sl.text("total", &fit::rupiah(d.total()), Tx::r(x2, tb).maxw(x2 - mx - 520.0));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s04");
    page(sl, d, Some((d.title.as_str(), "mute")));
    let plate = k("card") + 2.0 * k("pad");
    let wide = if have.qris { x2 - plate - k("col_gap") - mx } else { x2 - mx };
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let (bcap, ncap) = (sl.cap(&sl.face_of("bank"), sl.ty_size("bank")), sl.cap(&sl.face_of("number"), nsz));
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let stack = bcap + 40.0 + ncap + 44.0 + 62.0 + 70.0 + 14.0 + if notes.is_empty() { 0.0 } else { 40.0 + notes.len() as f32 * nlh };
    let top = (H - stack) / 2.0;
    // The stack stays clear of the top line.
    sl.need(top >= 140.0)?;
    sl.text("bank", &a.bank, Tx::l(mx, top + bcap).maxw(wide));
    let nb = top + bcap + 40.0 + ncap;
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    rule(sl, mx, nb + 44.0, wide);
    sl.text("atas", d.w.holder.as_str(), Tx::l(mx, nb + 44.0 + 62.0));
    let hb = nb + 44.0 + 62.0 + 70.0;
    sl.text("holder", &a.holder, Tx::l(mx, hb).size(hsz).maxw(wide));
    let mut yy = hb + 14.0 + 40.0 + nlh * 0.8;
    for line in &notes {
        sl.text("note", line, Tx::l(mx, yy).maxw(wide));
        yy += nlh;
    }
    if have.qris {
        // The code is dark on light, so it keeps a light plate on either ground.
        let (px, py) = (x2 - plate, (H - plate) / 2.0);
        sl.push(R::new(px, py, plate, plate).fill("white"));
        sl.image(Pic::Qris, px + k("pad"), py + k("pad"), k("card"), k("card"), 0.0, true, Pos::Mid);
        sl.text("caption", d.w.scan.as_str(), Tx::c(px + plate / 2.0, py + plate + 52.0).maxw(plate + 120.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref().map(|date| (date, "mute")));
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).cy().maxw(k("text_w")));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("title_base") - 62.0, sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, d, None);
    let wide = if have.photo { x2 - mx - k("photo_w") - k("text_gap") } else { x2 - mx };
    if have.photo {
        let (px, ph) = (x2 - k("photo_w"), k("photo_h"));
        // Two pictures stack; with one it is used twice, cropped from each end.
        let (first, second) = if have.photo2 { (Pos::Mid, Pos::Mid) } else { (Pos::Min, Pos::Max) };
        sl.image(Pic::Photo, px, top, k("photo_w"), ph, 0.0, false, first);
        sl.image(Pic::Photo2, px, top + ph + k("photo_gap"), k("photo_w"), ph, 0.0, false, second);
    }
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 2), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, top + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let mut y = top + cap + (lines.len() as f32 - 1.0) * tl + 56.0;
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.4).round();
    for para in d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect::<Vec<_>>() {
        for line in &para {
            sl.text("n_body", line, Tx::l(mx, y + blh * 0.8).maxw(wide));
            y += blh;
        }
        y += k("line_gap");
    }
    // A body taller than the page does not fit at this size.
    sl.need(y - k("line_gap") <= bottom)?;
    Ok(())
}
