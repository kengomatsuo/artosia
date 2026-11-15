//! Direction 13, Block: hard-edged flat blocks, a signal band with the title, a deep block beside it, near-black rows and a signal foot for the total.

use artosia_core::deck::{NoticeKind, Slide};

use super::data::{Data, Have};
use super::fit;
use super::{Fits, H, LINE, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_name_size, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The black ground, the signal band `band` high and the deep block from `blue_x` to the right edge of the band.
fn blocks(sl: &mut Sheet, band: f32, blue_x: f32) {
    sl.push(R::new(0.0, 0.0, W, H).fill("deep"));
    sl.push(R::new(0.0, 0.0, W, band).fill("signal"));
    if blue_x < W {
        sl.push(R::new(blue_x, 0.0, W - blue_x, band).fill("field2"));
    }
}

/// The blocks with the warta line, the title and its sub line in the band; returns where the deep block's words start.
fn page(sl: &mut Sheet, d: &Data, band: f32, blue_x: f32, title: &str, sub: Option<&str>) -> f32 {
    let mx = sl.g("mx");
    blocks(sl, band, blue_x);
    let room = blue_x - mx - 40.0;
    let left = sl.text("kicker", &d.church_kicker(""), Tx::l(mx, sl.g("kicker_cy")).cy().maxw(room - sl.mark_reserve()));
    sl.mark_after(left.x + left.w);
    sl.text("title", title, Tx::l(mx, band - sl.g("title_up")).cy().maxw(room));
    if let Some(sub) = sub.filter(|sub| !sub.is_empty()) {
        sl.text("sub", sub, Tx::l(mx, band - sl.g("sub_up")).cy().maxw(room));
    }
    blue_x + sl.g("blue_pad")
}

/// White words on the deep block: a small line at the top and a larger one under it.
fn on_blue(sl: &mut Sheet, left: f32, band: f32, small: Option<&str>, chip: Option<&str>) {
    let room = W - sl.g("mx") - left;
    if let Some(small) = small.filter(|small| !small.is_empty()) {
        sl.text("kicker", small, Tx::l(left, sl.g("kicker_cy")).cy().fill("white").maxw(room));
    }
    if let Some(chip) = chip.filter(|chip| !chip.is_empty()) {
        sl.text("chip", chip, Tx::l(left, band - sl.g("sub_up") - sl.g("chip_dy")).cy().maxw(room));
    }
}

/// A rule that is the mute colour at a share of its strength.
fn rule(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let height = sl.g("rule_w");
    let strength = sl.g("rule_op");
    sl.push(R::new(x, y - height / 2.0, width, height).fill("mute").op(strength));
}

/// The weekday and the rest of a long date, "Minggu" and "15 November 2026".
fn date_parts(d: &Data) -> (Option<&str>, Option<&str>) {
    match d.date_long.as_deref().and_then(|date| date.split_once(", ")) {
        Some((day, rest)) => (Some(day), Some(rest)),
        None => (None, d.date_long.as_deref()),
    }
}

/// The part of a long card, at the foot of the slide; deep where the foot is signal.
pub fn foot(sl: &mut Sheet, slide: &Slide, note: &str) {
    let colour = if slide.notice == NoticeKind::Money { "deep" } else { "mute" };
    sl.text("foot", note, Tx::l(sl.g("mx"), 1062.0).cy().fill(colour));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    let band = sl.g("band_h");
    let bx = page(sl, d, band, sl.g("blue_x"), &d.title, Some(&d.week_short));
    on_blue(sl, bx, band, Some(&d.w.wib), None);
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let gaps = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32 * k("gap");
    let avail = k("bot") - k("top");
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| k("event_w"), k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) + gaps <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min((avail - gaps) / n as f32);
    let mut ys = Vec::with_capacity(rows.len());
    let mut y = k("top");
    for (i, _) in rows.iter().enumerate() {
        if firsts[i] && i > 0 {
            y += k("gap");
        }
        ys.push(y);
        y += pitch;
    }
    // A day's signal cell spans its group of rows.
    for (i, r) in rows.iter().enumerate().filter(|(i, _)| firsts[*i]) {
        let last = (i..rows.len()).take_while(|at| *at == i || !firsts[*at]).last().unwrap_or(i);
        sl.push(R::new(mx, ys[i], k("day_w"), ys[last] + pitch - ys[i]).fill("signal"));
        sl.text("day", &r.day, Tx::l(mx + 20.0, ys[i] + pitch / 2.0).cy().fill("deep").maxw(k("day_w") - 40.0));
    }
    let (time_x, event_x) = (k("time_x"), k("event_x"));
    for (i, r) in rows.iter().enumerate() {
        let mid = ys[i] + pitch / 2.0;
        let time = Tx::l(time_x, mid).cy().maxw(event_x - time_x - 20.0);
        if r.special {
            sl.push(R::new(time_x - 20.0, ys[i], x2 - time_x + 20.0, pitch).fill("signal"));
        } else if i + 1 < rows.len() && !firsts[i + 1] && !rows[i + 1].special {
            rule(sl, time_x - 20.0, ys[i] + pitch, x2 - time_x + 20.0);
        }
        sl.text("time", &r.time, if r.special { time.fill("deep") } else { time });
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, mid, size, k("event_w"));
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, top, bottom, gutter) = (sl.g("mx"), sl.g("body_top"), sl.g("body_bot"), sl.g("gutter"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    let band = sl.g("band_h");
    let bx = page(sl, d, band, sl.g("blue_x"), &d.title, Some(&d.month));
    // The blue block always says which days: the highlight's beside highlighted names, else the page's own.
    let days = if d.highlighted() || d.covers { d.week_label.as_str() } else { d.month.as_str() };
    on_blue(sl, bx, band, None, (!days.is_empty()).then_some(days));
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = (W - 2.0 * mx - 2.0 * gutter) / 3.0;
    let avail = bottom - top;
    let day_w = names.iter().map(|x| sl.measure("bday_day", &d.day_of(x), None, None)).fold(sl.g("day_w"), f32::max);
    let (pad, day_gap) = (sl.g("row_pad"), sl.g("day_gap"));
    let name_w = colw - day_w - day_gap - 2.0 * pad;
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let hi = grown_size(k("size_max"), k("size_max") * k("grow"), avail, per, k("ratio"));
    let size = largest_name_size(sl, &list, "name_hl", name_w, hi, sl.g("name_floor")).ok_or(Split)?;
    let pitch = (avail / per as f32).min(k("pitch_max") * size);
    let x_of = |c: usize| mx + c as f32 * (colw + gutter);
    // The highlight is poured first, one block per run of highlighted names in a column.
    for c in 0..cols {
        let mut r = 0;
        while r < per {
            if !names.get(c * per + r).is_some_and(|nm| nm.hl) {
                r += 1;
                continue;
            }
            let from = r;
            while r < per && names.get(c * per + r).is_some_and(|nm| nm.hl) {
                r += 1;
            }
            sl.push(R::new(x_of(c), top + from as f32 * pitch + 2.0, colw, (r - from) as f32 * pitch - 4.0).fill("signal"));
        }
    }
    for (i, nm) in names.iter().enumerate() {
        let (cx, r) = (x_of(i / per), i % per);
        let cy = top + (r as f32 + 0.5) * pitch;
        let day = Tx::l(cx + pad, cy).cy();
        sl.text("bday_day", &d.day_of(nm), if nm.hl { day.fill("deep") } else { day });
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(role, &text, Tx::l(cx + pad + day_w + day_gap, cy).cy().size(size).maxw(name_w));
        let next_hl = names.get(i + 1).is_some_and(|next| next.hl);
        if r + 1 < per && i + 1 < n && !nm.hl && !next_hl {
            rule(sl, cx, cy + pitch / 2.0, colw);
        }
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("rows_top"), sl.g("rows_bot"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    let band = sl.g("band_h3");
    let bx = page(sl, d, band, sl.g("blue_x"), &d.title, Some(&d.w.offerings));
    let (day, date) = date_parts(d);
    on_blue(sl, bx, band, day, date);
    let avail = bottom - top;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let digits_w = funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), None, None)).fold(0.0, f32::max);
    let room = x2 - mx - digits_w - k("rp_gap") - rp_w - k("lab_gap");
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, |_| room, k("size_max"), sl.g("label_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    for (i, f) in funds.iter().enumerate() {
        let cy = top + (i as f32 + 0.5) * pitch;
        put_lines(sl, "fund", &cells[i], mx, cy, size, room);
        let amount = sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, cy).cy());
        sl.text("rp", "Rp", Tx::r(amount.x - k("rp_gap"), cy).cy());
        if i + 1 < funds.len() {
            rule(sl, mx, cy + pitch / 2.0, x2 - mx);
        }
    }
    // The signal foot carries the total, as large as the room the label leaves.
    let foot_top = sl.g("total_top");
    sl.push(R::new(0.0, foot_top, W, H - foot_top).fill("signal"));
    let cy = (foot_top + H) / 2.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(mx, cy).cy());
    let (total, rp) = (fit::digits(d.total()), sl.measure("total_rp", "Rp", None, None));
    let space = x2 - mx - label.w - sl.g("tot_gap") - rp - k("fig_gap");
    let z = sl.must(largest(sl.g("total_max"), sl.g("total_min"), |z| sl.measure("total", &total, Some(z), None) <= space), sl.g("total_min"))?;
    let figure = sl.text("total", &total, Tx::r(x2, cy).cy().size(z));
    sl.text("total_rp", "Rp", Tx::r(figure.x - k("fig_gap"), cy).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, band) = (sl.g("mx"), sl.g("band_h3"));
    let k = sl.sec("s04");
    let (blue_x, title) = (sl.g("blue_x"), if d.title.is_empty() { &d.w.account } else { &d.title });
    page(sl, d, band, blue_x, title, None);
    let wide = if have.qris { k("w_qr") } else { k("w") };
    let a = &d.account;
    let held = fit_cells(sl, "number", &[a.number.as_str()], |_| wide, k("number_max"), k("number_floor"), |_, _| true);
    let (nsz, lines, _) = sl.must(held, (k("number_floor"), vec![vec![a.number.clone()]], 0.0))?;
    let (bank_face, number_face, caption_face, holder_face, note_face) = (sl.face_of("bank"), sl.face_of("number"), sl.face_of("caption"), sl.face_of("holder"), sl.face_of("note"));
    let (bcap, ncap) = (sl.cap(&bank_face, sl.ty_size("bank")), sl.cap(&number_face, nsz));
    let (ccap, hcap) = (sl.cap(&caption_face, sl.ty_size("caption")), sl.cap(&holder_face, sl.ty_size("holder")));
    let number_h = ncap + (lines[0].len() as f32 - 1.0) * LINE * nsz;
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &note_face, note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let notes_h = if notes.is_empty() { 0.0 } else { k("note_gap") + notes.len() as f32 * nlh };
    let stack = bcap + k("bank_gap") + number_h + k("holder_gap") + ccap + k("label_gap") + hcap + notes_h;
    let mut y = band + (H - band - stack) / 2.0;
    // The stack stays clear of the band.
    sl.need(y >= band + 40.0)?;
    sl.text("bank", &a.bank, Tx::l(mx, y + bcap).maxw(wide));
    y += bcap + k("bank_gap");
    for (at, line) in lines[0].iter().enumerate() {
        sl.text("number", line, Tx::l(mx, y + ncap + at as f32 * LINE * nsz).size(nsz).maxw(wide));
    }
    y += number_h + k("holder_gap");
    sl.text("caption", &d.w.holder, Tx::l(mx, y + ccap).maxw(wide));
    y += ccap + k("label_gap");
    sl.text("holder", &a.holder, Tx::l(mx, y + hcap).maxw(wide));
    y += hcap + k("note_gap");
    for line in &notes {
        sl.text("note", line, Tx::l(mx, y + nlh * 0.8).maxw(wide));
        y += nlh;
    }
    if have.qris {
        // The deep block runs down the right edge and holds the code on a white plate.
        sl.push(R::new(blue_x, band, W - blue_x, H - band).fill("field2"));
        let (plate, pad) = (k("plate"), k("pad"));
        let (px, py) = (blue_x + (W - blue_x - plate) / 2.0, band + (H - band - plate) / 2.0 - 30.0);
        sl.push(R::new(px, py, plate, plate).fill("white"));
        sl.image(Pic::Qris, px + pad, py + pad, plate - 2.0 * pad, plate - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(blue_x + (W - blue_x) / 2.0, py + plate + 56.0).fill("white").maxw(W - blue_x - 2.0 * mx));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, band) = (sl.g("mx"), sl.g("band_h3"));
    let k = sl.sec("s05");
    let bx = page(sl, d, band, k("blue_x"), &d.w.verse, None);
    on_blue(sl, bx, band, d.date_long.as_deref(), None);
    sl.text("ref", &d.reference, Tx::l(bx, band - sl.g("title_up")).cy().maxw(W - mx - bx));
    if have.photo {
        sl.image(Pic::Photo, W - k("pic_w"), band, k("pic_w"), H - band, 0.0, false, Pos::Mid);
    }
    let room = k("bot") - k("top");
    sl.push(R::new(mx, k("top"), k("bar_w"), room).fill("signal"));
    let face = sl.face_of("verse");
    let wide = if have.photo { k("w_pic") } else { k("w") };
    verse(sl, &d.verses, &VerseBox { x: mx + k("bar_w") + k("bar_gap"), top: k("top"), w: wide, room, font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom, ky) = (sl.g("mx"), sl.g("x2"), sl.g("body_bot"), sl.g("kicker_cy"));
    let k = sl.sec("s06");
    let wide = if have.photo { k("w_pic") } else { k("w") };
    let (face, ls) = (sl.face_of("n_title"), sl.ty("n_title").ls);
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest(k("title_max"), k("title_floor"), |z| wrap_at(z).len() <= 3), k("title_floor"))?;
    let lines = wrap_at(z);
    let (cap, tl) = (sl.cap(&face, z), k("title_lh") * z);
    let band = k("title_top") + cap + (lines.len().max(1) - 1) as f32 * tl + k("pad");
    blocks(sl, band, W);
    let left = sl.text("kicker", &d.church_kicker(""), Tx::l(mx, ky).cy().maxw(x2 - mx - sl.mark_reserve()));
    sl.mark_after(left.x + left.w);
    if let Some(date) = d.date_long.as_deref() {
        sl.text("kicker", date, Tx::r(x2, ky).cy().maxw(x2 - mx - left.w - 40.0));
    }
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("title_top") + cap + i as f32 * tl).size(z).maxw(wide));
    }
    if have.photo {
        sl.image(Pic::Photo, W - k("pic_w"), band, k("pic_w"), H - band, 0.0, false, Pos::Mid);
    }
    let (body_face, body_size) = (sl.face_of("n_body"), sl.ty_size("n_body"));
    let blh = (body_size * 1.4).round();
    let mut y = band + k("gap_body");
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
