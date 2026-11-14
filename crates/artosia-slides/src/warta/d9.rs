//! Direction 9, Civic board: a departure board with a header band, mono figures, condensed words and an inverted row for the highlight.

use artosia_core::deck::{NoticeKind, Slide};

use super::grown_size;
use super::data::{Data, Have};
use super::fit;
use super::{Fits, H, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, largest, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The ground and the header band: the warta line at the left, `right` at the right of it, the title and the subtitle under them.
fn page(sl: &mut Sheet, d: &Data, right: Option<&str>, title: &str, sub: Option<&str>) {
    let (mx, x2, ky, base) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_y"), sl.g("title_base"));
    sl.push(R::new(0.0, 0.0, W, H).fill("paper"));
    sl.push(R::new(0.0, 0.0, W, sl.g("band_h")).fill("band"));
    let left = sl.text("kicker", &d.w.warta, Tx::l(mx, ky).maxw(x2 - mx));
    if let Some(right) = right.filter(|right| !right.is_empty()) {
        sl.text("kicker", right, Tx::r(x2, ky).maxw(x2 - mx - left.w - 40.0));
    }
    let t = sl.text("title", title, Tx::l(mx, base).maxw(x2 - mx));
    let room = x2 - mx - t.w - 40.0;
    if let Some(sub) = sub.filter(|sub| !sub.is_empty() && sl.measure("subtitle", sub, Some(32.0), None) <= room) {
        sl.text("subtitle", sub, Tx::r(x2, base).maxw(room));
    }
}

fn rule(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let height = sl.g("hair");
    sl.push(R::new(x, y - height / 2.0, width, height).fill("rule"));
}

/// An inverted rectangle, the board's highlight.
fn inverted(sl: &mut Sheet, x: f32, y: f32, width: f32, height: f32) {
    sl.push(R::new(x, y, width, height).fill("band"));
}

/// The part of a long card, at the foot of the slide: at the left, or at the right when the month holds the left.
pub fn foot(sl: &mut Sheet, slide: &Slide, note: &str) {
    if slide.notice == NoticeKind::Birthdays {
        sl.text("caption", note, Tx::r(sl.g("x2"), 1050.0));
    } else {
        sl.text("foot", note, Tx::l(sl.g("mx"), 1050.0).fill("ink"));
    }
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    let church = (!d.church.is_empty()).then_some(d.church.as_str());
    page(sl, d, church, &d.title, Some(&d.week_short));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let gaps = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32 * k("group_gap");
    let (event_x, ratio) = (k("event_x"), k("ratio"));
    // A special row keeps room for its tag, so every row is measured in that room.
    let ev_w = x2 - event_x - if rows.iter().any(|r| r.special) { k("tag_room") } else { k("ev_pad") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, k("size_max"), k("size_floor"), |z, e| n as f32 * (ratio * z + e) + gaps <= bottom - top).ok_or(Split)?;
    let pitch = ratio * size + extra;
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += k("group_gap");
        }
        let mid = y + pitch / 2.0;
        if r.special {
            inverted(sl, mx - pad, y, x2 - mx + 2.0 * pad, pitch);
        } else {
            rule(sl, mx, y + pitch, x2 - mx);
        }
        if firsts[i] {
            let label = if r.date > 0 { format!("{} {}", r.dow, r.date) } else { r.dow.clone() };
            let at = Tx::l(k("day_x"), mid).cy().maxw(k("time_x") - k("day_x") - 20.0);
            sl.text("day_name", &label, if r.special { at.fill("band_ink") } else { at });
        }
        sl.text(if r.special { "time_hl" } else { "time" }, &r.time, Tx::l(k("time_x"), mid).cy().maxw(event_x - k("time_x") - 20.0));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, mid, size, ev_w);
        if r.special {
            sl.text("tag", &d.w.special, Tx::r(x2, mid).cy().maxw(k("tag_room") - k("ev_pad")));
        }
        y += pitch;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, d, (!d.church.is_empty()).then_some(d.church.as_str()), &d.title, d.highlighted().then_some(d.week_label.as_str()));
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let mon_w = if d.spans { names.iter().map(|x| sl.measure("bday_mon", &x.month, None, None)).fold(0.0, f32::max) } else { 0.0 };
    let day_cells = |sl: &Sheet, z: f32| sl.measure("bday_day_hl", "00", Some(z), None) + if d.spans { k("day_gap") + mon_w } else { 0.0 };
    let role_of = |hl: bool| if hl { "name_hl" } else { "name" };
    let name_w = |sl: &Sheet, z: f32| colw - day_cells(sl, z) - k("name_gap");
    let hi = grown_size(k("size_max"), k("size_max") * 1.7, avail, per, k("ratio"));
    let holds = |z: f32| names.iter().all(|nm| fit::ladder(&nm.name, name_w(sl, z), &|s| sl.measure(role_of(nm.hl), s, Some(z), None)).is_some());
    let size = largest(hi, k("size_floor"), holds).ok_or(Split)?;
    let pitch = (avail / per as f32).min(k("ratio") * 1.35 * size);
    let num_w = sl.measure("bday_day_hl", "00", Some(size), None);
    let x_of = |c: usize| mx + c as f32 * (colw + k("col_gap"));
    for i in 0..names.len() {
        rule(sl, x_of(i / per), top + ((i % per) as f32 + 1.0) * pitch, colw);
    }
    // One inverted rectangle per column, from its first highlighted name to its last, so a run over a column break is two.
    for c in 0..cols {
        let hl: Vec<usize> = (0..per).filter(|r| names.get(c * per + r).is_some_and(|nm| nm.hl)).collect();
        if let (Some(first), Some(last)) = (hl.first(), hl.last()) {
            inverted(sl, x_of(c) - k("band_over"), top + *first as f32 * pitch, colw + 2.0 * k("band_over"), (last - first + 1) as f32 * pitch);
        }
    }
    let cap = sl.cap(&sl.face_of("name"), size);
    for (i, nm) in names.iter().enumerate() {
        let (cx, r) = (x_of(i / per), i % per);
        let base = top + r as f32 * pitch + pitch / 2.0 + cap / 2.0;
        let role = role_of(nm.hl);
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &format!("{:02}", nm.day), Tx::r(cx + num_w, base).size(size - 4.0));
        if d.spans {
            sl.text(if nm.hl { "bday_mon_hl" } else { "bday_mon" }, &nm.month, Tx::l(cx + num_w + k("day_gap"), base));
        }
        let (text, _) = name_in(sl, &nm.name, role, name_w(sl, size), size);
        sl.text(role, &text, Tx::l(cx + day_cells(sl, size) + k("name_gap"), base).size(size).maxw(name_w(sl, size)));
    }
    sl.text("caption", &d.month, Tx::l(mx, 1050.0));
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, Some(d.w.offerings.as_str()), &d.title, d.date_long.as_deref());
    let total_y = bottom - k("total_h");
    let avail = total_y - k("total_gap") - top;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let amount_w = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z - 6.0), None)).fold(0.0, f32::max);
    let label_room = |sl: &Sheet, z: f32| x2 - mx - rp_w - k("rp_gap") - amount_w(sl, z) - 48.0;
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, |z| label_room(sl, z), k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let pitch = (1.9 * size + extra).min(avail / n as f32);
    let (col, room) = (amount_w(sl, size), label_room(sl, size));
    for (i, f) in funds.iter().enumerate() {
        let mid = top + i as f32 * pitch + pitch / 2.0;
        rule(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx);
        put_lines(sl, "fund", &cells[i], mx, mid, size, room);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, mid).cy().size(size - 6.0));
        sl.text("rp", "Rp", Tx::r(x2 - col - k("rp_gap"), mid).cy());
    }
    inverted(sl, mx - pad, total_y, x2 - mx + 2.0 * pad, k("total_h"));
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(mx, total_y + 52.0));
    let (total, rp) = (fit::digits(d.total()), sl.measure("total_rp", "Rp", None, None));
    let room = x2 - mx - label.w - 48.0;
    let fits = |z: f32| sl.measure("total", &total, Some(z), None) + rp + 20.0 <= room;
    let tz = largest(sl.ty_size("total"), 64.0, fits).ok_or(Split)?;
    let base = total_y + k("total_h") - 36.0;
    let at = sl.text("total", &total, Tx::r(x2, base).size(tz));
    sl.text("total_rp", "Rp", Tx::r(at.x - 20.0, base));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s04");
    page(sl, d, Some(d.w.account.as_str()), &d.title, None);
    let wide = if have.qris { x2 - mx - k("card") - k("col_gap") } else { x2 - mx };
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).ok_or(Split)?;
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).ok_or(Split)?;
    let y = k("stack_top");
    sl.text("bank", &a.bank, Tx::l(mx, y).maxw(wide));
    let nb = y + 34.0 + 0.7 * nsz;
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    sl.push(R::new(mx, nb + 36.0, wide, 3.0).fill("band"));
    sl.text("atas", &d.w.holder, Tx::l(mx, nb + 36.0 + 62.0));
    sl.text("holder", &a.holder, Tx::l(mx, nb + 36.0 + 62.0 + 72.0).size(hsz).maxw(wide));
    let mut ny = nb + 36.0 + 62.0 + 72.0 + 64.0;
    if !a.note.is_empty() {
        let (face, size) = (sl.face_of("note"), sl.ty_size("note"));
        for line in paragraphs(sl, &a.note, &face, size, wide) {
            sl.text("note", &line, Tx::l(mx, ny).maxw(wide));
            ny += 54.0;
        }
    }
    sl.need(ny - 54.0 <= bottom)?;
    if have.qris {
        // The code is dark on light, so it keeps a light plate on every ground.
        let (qx, qy, card, pad) = (x2 - k("card"), y, k("card"), k("card_pad"));
        sl.push(R::new(qx, qy, card, card).fill("card"));
        sl.image(Pic::Qris, qx + pad, qy + pad, card - 2.0 * pad, card - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(qx + card / 2.0, qy + card + 56.0).maxw(card + 120.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref(), &d.w.verse, None);
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(k("text_w")));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, d, d.date_long.as_deref(), &d.w.notice, None);
    let wide = if have.photo { x2 - mx - k("photo_w") - k("photo_gap") } else { k("text_w") };
    if have.photo {
        sl.image(Pic::Photo, x2 - k("photo_w"), k("photo_top"), k("photo_w"), k("photo_h"), 0.0, false, Pos::Mid);
    }
    let (face, ls) = (sl.face_of("n_title"), sl.ty("n_title").ls);
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let (body_face, body_size) = (sl.face_of("n_body"), sl.ty_size("n_body"));
    let blh = (body_size * 68.0 / 52.0).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let lines_in_body: usize = body.iter().map(Vec::len).sum();
    let (top, tl, gap) = (k("top"), k("title_lh"), k("line_gap"));
    let stack = |z: f32| top + sl.cap(&face, z) + (wrap_at(z).len().max(1) - 1) as f32 * tl * z + 130.0 + lines_in_body as f32 * blh + body.len() as f32 * gap;
    let z = sl.must(largest(k("title_max"), k("title_floor"), |z| wrap_at(z).len() <= 2 && stack(z) <= bottom), k("title_floor"))?;
    let lines = wrap_at(z);
    let cap = sl.cap(&face, z);
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, top + cap + i as f32 * tl * z).size(z).maxw(wide));
    }
    let mut y = top + cap + (lines.len().max(1) - 1) as f32 * tl * z + 48.0;
    sl.push(R::new(mx, y, 144.0, 6.0).fill("band"));
    y += 6.0 + 76.0;
    for para in &body {
        for line in para {
            sl.text("n_body", line, Tx::l(mx, y).maxw(wide));
            y += blh;
        }
        y += gap;
    }
    // A body taller than the page does not fit at this size.
    sl.need(y <= bottom)?;
    Ok(())
}
