//! Direction 2, Pill poster: a warm grey ground, near-black and white pills, one yellow and one orange sticker.

use artosia_core::deck::NoticeKind;

use super::largest_step;
use super::data::{self as w, Data, Have};
use super::fit;
use super::{Fits, H, fit_cells, put_lines, Pic, Pos, R, Sheet, Split, Tx, W, largest, largest_name_size, name_in, paragraph, paragraphs, verse, VerseBox};

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

fn sticker(sl: &mut Sheet, cx: f32, cy: f32, dia: f32, label: &str, deg: f32) {
    sl.push(R::new(cx - dia / 2.0, cy - dia / 2.0, dia, dia).fill("orange").pill());
    sl.push(R::new(cx - dia / 2.0 + 10.0, cy - dia / 2.0 + 10.0, dia - 20.0, dia - 20.0).pill().border(3.0, "ink").bop(0.35));
    sl.spin("sticker", label, cx, cy, deg, None);
}

/// The kicker pills at the top; returns the x where the next one would start.
fn head(sl: &mut Sheet, kicker: &str, second: Option<&str>, title: Option<&str>) {
    let (mx, pad, py, ph) = (sl.g("mx"), sl.g("pad"), sl.g("pill_y"), sl.g("pill_h"));
    sl.push(R::new(0.0, 0.0, W, H).fill("ground"));
    let w1 = sl.measure("tag", kicker, None, None) + 2.0 * pad;
    sl.push(R::new(mx, py, w1, ph).fill("pill").pill());
    sl.text("tag", kicker, Tx::l(mx + pad, py + ph / 2.0).cy());
    if let Some(second) = second {
        let w2 = sl.measure("tag_ink", second, None, None) + 2.0 * pad;
        let x = mx + w1 + 14.0;
        sl.push(R::new(x, py, w2, ph).fill("yellow").pill());
        sl.text("tag_ink", second, Tx::l(x + pad, py + ph / 2.0).cy());
    }
    if let Some(title) = title {
        sl.text("title", title, Tx::l(mx, sl.g("title_base")).maxw(sl.g("x2") - mx));
    }
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, row_gap) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("row_gap"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    head(sl, w::WARTA, (!d.week_short.is_empty()).then_some(d.week_short.as_str()), Some(&d.title));
    let avail = bottom - top;
    let sparse = n as f32 <= k("sparse_rows");
    let pitch = (avail / n as f32).min(if sparse { k("sparse_h") } else { 999.0 } + row_gap);
    let h = pitch - row_gap;
    let y0 = if sparse { top + (avail - n as f32 * pitch) / 2.0 } else { top };
    let ev_x = mx + k("day_w") + row_gap;
    let pw = x2 - ev_x;
    let text_x = ev_x + 38.0 + k("time_w") + 2.0 * k("div_gap") + 4.0;
    let room = pw - (text_x - ev_x) - 40.0 - k("sticker_d") * 0.62;
    let hi = if sparse { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, _) = fit_cells(sl, "event_hl", &events, |_| room, hi, k("size_floor"), |z, e| z * 1.4 + e <= h).ok_or(Split)?;
    let mut seen: Vec<&str> = Vec::new();
    let mut stickers = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let y = y0 + i as f32 * pitch;
        let cy = y + h / 2.0;
        let again = seen.contains(&r.day.as_str());
        seen.push(&r.day);
        let date = if r.date > 0 { r.date.to_string() } else { String::new() };
        if again {
            sl.push(R::new(mx, y, k("day_w"), h).pill().border(4.0, "ink"));
            sl.text("day_out", &r.day, Tx::l(mx + 34.0, cy).cy().maxw(200.0));
            sl.text("day_out_n", &date, Tx::r(mx + k("day_w") - 34.0, cy).cy());
        } else {
            sl.push(R::new(mx, y, k("day_w"), h).fill("pill").pill());
            sl.text("day", &r.day, Tx::l(mx + 34.0, cy).cy().maxw(200.0));
            sl.text("day_n", &date, Tx::r(mx + k("day_w") - 34.0, cy).cy());
        }
        sl.push(R::new(ev_x, y, pw, h).fill(if r.special { "yellow" } else { "white" }).pill());
        sl.text("time", &r.time, Tx::l(ev_x + 38.0, cy).cy().size(46.0_f32.min(size)));
        sl.push(R::new(text_x - k("div_gap") - 4.0, cy - h * 0.26, 4.0, h * 0.52).fill("ink").op(0.35));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], text_x, cy, size, if r.special { room } else { pw - (text_x - ev_x) - 40.0 });
        if r.special {
            stickers.push((x2 - k("sticker_d") / 2.0 + 12.0, cy));
        }
    }
    // Last, so no later row covers one.
    for (cx, cy) in stickers {
        sticker(sl, cx, cy, k("sticker_d"), w::SPECIAL, -10.0);
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base, pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"), sl.g("pad"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    head(sl, w::WARTA, Some(&d.month), Some(&d.title));
    if let Some((from, to)) = d.week {
        let week = format!("{} {}\u{2013}{}", w::THIS_WEEK, from, to);
        let ww = sl.measure("tag_ink", &week, None, None) + 2.0 * pad;
        let cap = 0.7 * sl.ty_size("title");
        sl.push(R::new(x2 - ww, base - cap / 2.0 - 32.0, ww, 64.0).fill("orange").pill());
        sl.text("tag_ink", &week, Tx::l(x2 - ww + pad, base - cap / 2.0).cy());
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let sparse = n as f32 <= k("sparse_names");
    let cw = if sparse { k("sparse_w") } else { x2 - mx };
    let colw = (cw - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let pitch = (avail / per as f32).min(if sparse { k("sparse_h") } else { k("pitch_max") });
    let rh = (pitch - 10.0).min(if sparse { 100.0 } else { 66.0 });
    let pw = colw - k("circle") - k("circle_gap");
    let name_w = pw - 2.0 * k("pill_pad");
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let size = largest_name_size(sl, &list, "name_hl", name_w, if sparse { k("sparse_size") } else { k("size_max") }, k("size_floor")).ok_or(Split)?;
    let circ = if sparse { 84.0 } else { k("circle") };
    let y0 = if sparse { top + (avail - per as f32 * pitch) / 2.0 } else { top };
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let cy = y0 + r as f32 * pitch + pitch / 2.0;
        sl.push(R::new(cx, cy - circ / 2.0, circ, circ).fill(if nm.hl { "orange" } else { "pill" }).pill());
        sl.text(if nm.hl { "dot_hl" } else { "dot" }, &format!("{:02}", nm.day), Tx::c(cx + circ / 2.0, cy).cy().size(if sparse { 44.0 } else { 36.0 }));
        if nm.hl {
            sl.push(R::new(cx + circ + k("circle_gap"), cy - rh / 2.0, pw, rh).fill("yellow").pill());
        }
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(role, &text, Tx::l(cx + circ + k("circle_gap") + k("pill_pad"), cy).cy().size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, row_gap) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("row_gap"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    head(sl, w::WARTA, d.date_long.as_deref(), Some(&d.title));
    let avail = bottom - top - k("total_h") - k("total_gap");
    let pitch = (avail / n as f32).min(k("pitch_max"));
    let rh = pitch - row_gap;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let dw_at = |z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let ap_at = |z: f32| 2.0 * k("amount_pad") + rp_w + k("rp_gap") + dw_at(z);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, _) = fit_cells(sl, "fund", &labels, |z| x2 - mx - ap_at(z) - row_gap - 80.0, k("size_max"), k("size_floor"), |z, e| z * 1.45 + e <= rh).ok_or(Split)?;
    let ap_w = ap_at(size);
    let label_w = x2 - mx - ap_w - row_gap - 80.0;
    for (i, f) in funds.iter().enumerate() {
        let y = top + i as f32 * pitch;
        let cy = y + rh / 2.0;
        sl.push(R::new(mx, y, x2 - mx - ap_w - row_gap, rh).fill("white").pill());
        put_lines(sl, "fund", &cells[i], mx + 40.0, cy, size, label_w);
        let ax = x2 - ap_w;
        sl.push(R::new(ax, y, ap_w, rh).fill("pill").pill());
        sl.text("rp", "Rp", Tx::l(ax + k("amount_pad"), cy).cy());
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2 - k("amount_pad"), cy).cy().size(size));
    }
    let (th, ty) = (k("total_h"), bottom - k("total_h"));
    sl.push(R::new(mx, ty, x2 - mx, th).fill("yellow").pill());
    sl.text("total_label", w::TOTAL, Tx::l(mx + 52.0, ty + th / 2.0).cy());
    let tv = fit::digits(d.total());
    let tw = sl.measure("total", &tv, None, None);
    let trp = sl.measure("total_rp", "Rp", None, None);
    let iw = 2.0 * 44.0 + trp + 18.0 + tw;
    let ix = x2 - 14.0 - iw;
    sl.push(R::new(ix, ty + 14.0, iw, th - 28.0).fill("pill").pill());
    sl.text("total_rp", "Rp", Tx::l(ix + 44.0, ty + th / 2.0).cy());
    sl.text("total", &tv, Tx::r(x2 - 14.0 - 44.0, ty + th / 2.0).cy().maxw(iw));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, pad) = (sl.g("mx"), sl.g("pad"));
    let k = sl.sec("s04");
    head(sl, w::WARTA, None, None);
    let a = &d.account;
    let (left, gap) = (if have.qris { k("left_w") } else { sl.g("x2") - mx }, k("gap"));
    let nsz = largest_step(k("number_max"), k("number_floor"), 2.0, |z| sl.measure("number", &a.number, Some(z), None) <= left - 112.0).unwrap_or(k("number_floor"));
    let note_size = sl.ty_size("note");
    let lines = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, left - 2.0 * k("note_pad")) };
    let nlh = (note_size * 1.38).round();
    let note_h = if lines.is_empty() { 0.0 } else { lines.len() as f32 * nlh + 60.0 };
    let blocks = if lines.is_empty() { 2.0 } else { 3.0 };
    let total = k("bank_h") + k("number_h") + k("holder_h") + note_h + blocks * gap;
    let (py, ph, bottom) = (sl.g("pill_y"), sl.g("pill_h"), sl.g("body_bottom"));
    let mut y = py + ph + 36.0 + (bottom - py - ph - 36.0 - total) / 2.0;
    sl.push(R::new(mx, y, left, k("bank_h")).fill("white").pill());
    sl.text("bank", &a.bank, Tx::l(mx + 48.0, y + k("bank_h") / 2.0).cy().maxw(left - 96.0));
    y += k("bank_h") + gap;
    sl.push(R::new(mx, y, left, k("number_h")).fill("pill").pill());
    sl.text("number", &a.number, Tx::c(mx + left / 2.0, y + k("number_h") / 2.0).cy().size(nsz).maxw(left - 112.0));
    y += k("number_h") + gap;
    let hlw = sl.measure("holder_l", w::HOLDER_LABEL, None, None) + 60.0;
    sl.push(R::new(mx, y, left, k("holder_h")).fill("white").pill());
    sl.push(R::new(mx + 8.0, y + 8.0, hlw, k("holder_h") - 16.0).fill("pill").pill());
    sl.text("holder_l", w::HOLDER_LABEL, Tx::l(mx + 38.0, y + k("holder_h") / 2.0).cy());
    let hx = mx + 8.0 + hlw + 30.0;
    let hsz = largest(sl.ty_size("holder"), 40.0, |z| sl.measure("holder", &a.holder, Some(z), None) <= mx + left - 36.0 - hx).unwrap_or(40.0);
    sl.text("holder", &a.holder, Tx::l(hx, y + k("holder_h") / 2.0).cy().size(hsz).maxw(mx + left - 36.0 - hx));
    y += k("holder_h") + gap;
    if !lines.is_empty() {
        sl.push(R::new(mx, y, left, note_h).fill("yellow").rx(44.0));
        for (i, line) in lines.iter().enumerate() {
            sl.text("note", line, Tx::l(mx + k("note_pad"), y + 30.0 + (i as f32 + 0.5) * nlh).cy().maxw(left - 2.0 * k("note_pad")));
        }
    }
    if have.qris {
        let card = k("card");
        let cy0 = (bottom + py + ph) / 2.0 - card / 2.0;
        sl.push(R::new(k("card_x"), cy0, card, card).fill("white").rx(56.0));
        sl.image(Pic::Qris, k("card_x"), cy0, card, card, 56.0, true, Pos::Mid);
        let cw = sl.measure("caption", w::QRIS_CAPTION, None, None) + 2.0 * pad;
        sl.push(R::new(k("card_x") + card / 2.0 - cw / 2.0, cy0 + card - 28.0, cw, 64.0).fill("pill").pill());
        sl.text("caption", w::QRIS_CAPTION, Tx::c(k("card_x") + card / 2.0, cy0 + card + 4.0).cy());
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s05");
    head(sl, w::WARTA, Some(w::VERSE_KICKER), None);
    let (cx0, cy0) = (mx, k("card_y"));
    let (cw, ch) = (x2 - mx, bottom - k("card_y"));
    sl.push(R::new(cx0, cy0, cw, ch).fill("white").rx(64.0));
    let face = sl.face_of("verse");
    let top = cy0 + k("ref_h") / 2.0 + 44.0;
    let room = cy0 + ch - k("card_pad") + 20.0 - top;
    verse(sl, &d.verses, &VerseBox { x: cx0 + k("card_pad"), top, w: cw - 2.0 * k("card_pad"), room, font: &face, section: "s05" })?;
    let rw = sl.measure("ref", &d.reference, None, None) + 96.0;
    sl.push(R::new(cx0 + 56.0, cy0 - k("ref_h") / 2.0, rw, k("ref_h")).fill("pill").pill());
    sl.text("ref", &d.reference, Tx::l(cx0 + 56.0 + 48.0, cy0).cy());
    let sd = k("sticker_d");
    let qx = cx0 + cw - sd / 2.0 - 70.0;
    sl.push(R::new(qx - sd / 2.0, cy0 - sd / 2.0, sd, sd).fill("orange").pill());
    sl.text("quote", "\u{201d}", Tx::c(qx, cy0 + k("quote_drop")));
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, py, ph, bottom) = (sl.g("mx"), sl.g("pill_y"), sl.g("pill_h"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    head(sl, w::WARTA, Some(w::NOTICE_KICKER), None);
    let wide = k("text_w");
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title.to_uppercase());
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.34).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide - 80.0)).collect();
    let card_h = body.iter().map(Vec::len).sum::<usize>() as f32 * blh + body.len().saturating_sub(1) as f32 * 18.0 + 72.0;
    let room = bottom - (py + ph + 40.0) - card_h - 36.0;
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, 0.0);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3 && wrap_at(z).len() as f32 * z * k("title_lh") <= room);
    let z = sl.must(z, k("title_floor"))?;
    let lines = wrap_at(z);
    let ty = py + ph + 40.0;
    if !have.photo {
        let dd = k("disc_d");
        sl.push(R::new(k("disc_x"), k("disc_y"), dd, dd).fill("orange").pill());
        sl.push(R::new(k("disc_x") + 24.0, k("disc_y") + 24.0, dd - 48.0, dd - 48.0).pill().border(6.0, "ink").bop(0.35));
        sl.spin("sticker", w::STICKER, k("disc_x") + dd / 2.0, k("disc_y") + dd / 2.0, -10.0, Some(k("disc_text")));
    }
    let cap = sl.cap(&face, z);
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, ty + (i as f32 + 0.5) * z * k("title_lh") + cap / 2.0).size(z).maxw(wide));
    }
    let cy = ty + lines.len() as f32 * z * k("title_lh") + 36.0;
    sl.push(R::new(mx, cy, wide, card_h).fill("white").rx(44.0));
    let mut yy = cy + 36.0;
    for (pi, para) in body.iter().enumerate() {
        for line in para {
            sl.text(if pi == 0 { "n_body_b" } else { "n_body" }, line, Tx::l(mx + 40.0, yy + blh / 2.0).cy().maxw(wide - 80.0));
            yy += blh;
        }
        yy += 18.0;
    }
    if have.photo {
        sl.image(Pic::Photo, k("photo_x"), k("photo_y"), k("photo_w"), k("photo_h"), k("photo_w") / 2.0, false, Pos::Mid);
        sticker(sl, k("photo_x") + 24.0, k("photo_y") + k("photo_h") - 150.0, k("sticker_d"), w::STICKER, -12.0);
    }
    Ok(())
}
