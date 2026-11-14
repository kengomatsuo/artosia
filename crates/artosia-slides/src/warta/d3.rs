//! Direction 3, Neon night: a navy ground, neon rings and one neon outline per highlight, no blur.

use artosia_core::deck::NoticeKind;

use super::d1::largest_step;
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

/// A neon outline without blur: the border, and wider borders of the same colour at low opacity.
#[allow(clippy::too_many_arguments)] // a box, its corner, colour, width and turn, as the layouts name them
fn neon(sl: &mut Sheet, x: f32, y: f32, wide: f32, high: f32, rx: f32, colour: &str, bw: f32, rot: f32) {
    for glow in sl.list("grid", "glow") {
        let (off, gw, op) = (glow[0], glow[1], glow[2]);
        sl.push(R::new(x - off, y - off, wide + 2.0 * off, high + 2.0 * off).rx(if rx > 0.0 { rx + off } else { 0.0 }).border(gw, colour).bop(op).rot(rot));
    }
    sl.push(R::new(x, y, wide, high).rx(rx).border(bw, colour).rot(rot));
}

fn deco(sl: &mut Sheet, sid: &str) {
    let items = sl.d["deco"][sid].as_array().cloned().unwrap_or_default();
    for item in items {
        let (kind, x, y, dia, colour) = (item[0].as_str().unwrap_or(""), item[1].as_f64().unwrap_or(0.0) as f32, item[2].as_f64().unwrap_or(0.0) as f32, item[3].as_f64().unwrap_or(0.0) as f32, item[4].as_str().unwrap_or("pink").to_string());
        match kind {
            "ring" => neon(sl, x, y, dia, dia, dia / 2.0, &colour, 10.0, 0.0),
            "diamond" => neon(sl, x, y, dia, dia, 10.0, &colour, 8.0, 45.0),
            _ => sl.push(R::new(x, y, dia, dia).fill(&colour).pill()),
        }
    }
}

fn page(sl: &mut Sheet, sid: &str, kicker: &str, title: Option<&str>, pills: &[(String, &str, &str)]) {
    let (mx, ky, py, ph) = (sl.g("mx"), sl.g("kicker_y"), sl.g("pill_y"), sl.g("pill_h"));
    sl.push(R::new(0.0, 0.0, W, H).fill("grad:ground"));
    deco(sl, sid);
    sl.text("kicker", kicker, Tx::l(mx, ky).cy().maxw(1100.0));
    if let Some(title) = title {
        sl.text("title", title, Tx::l(mx, sl.g("title_base")).maxw(1300.0));
    }
    let mut x = mx;
    for (txt, colour, role) in pills {
        let wide = sl.measure(role, txt, None, None) + 60.0;
        neon(sl, x, py, wide, ph, ph / 2.0, colour, 4.0, 0.0);
        sl.text(role, txt, Tx::l(x + 30.0, py + ph / 2.0).cy());
        x += wide + 22.0;
    }
}

fn line(sl: &mut Sheet, x: f32, y: f32, wide: f32) {
    let lw = sl.g("line_w");
    sl.push(R::new(x, y, wide, lw).fill("line"));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    let pills = if d.week_short.is_empty() { Vec::new() } else { vec![(d.week_short.clone(), "yellow", "pill_t")] };
    page(sl, "s01", &d.kicker(w::WIB), Some(&d.title), &pills);
    let avail = bottom - top;
    let sparse = n as f32 <= k("sparse_rows");
    let ev_w = x2 - k("event_x") - k("tag_room");
    let hi = if sparse { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let y0 = if sparse { top + (avail - n as f32 * pitch) / 2.0 } else { top };
    let cap = sl.cap(&sl.face_of("event"), size);
    for (i, r) in rows.iter().enumerate() {
        let y = y0 + i as f32 * pitch;
        let cy = y + pitch / 2.0;
        let by = cy + cap / 2.0;
        if r.special {
            neon(sl, mx - k("hl_pad"), y + 5.0, x2 - mx + 2.0 * k("hl_pad"), pitch - 10.0, 26.0, "yellow", 5.0, 0.0);
        } else if i + 1 < rows.len() && !rows[i + 1].special {
            line(sl, mx, y + pitch, x2 - mx);
        }
        let dn = sl.text("day", &r.dow, Tx::l(k("day_x"), by));
        if r.date > 0 {
            let at = dn.x + dn.w + 16.0;
            sl.text("day_n", &r.date.to_string(), Tx::l(at, by).maxw(k("time_x") - 20.0 - at));
        }
        sl.text("time", &r.time, Tx::l(k("time_x"), by));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], k("event_x"), cy, size, ev_w);
        if r.special {
            let tw = sl.measure("tag", w::SPECIAL, None, None) + 52.0;
            let tx = x2 + k("hl_pad") - 20.0 - tw;
            sl.push(R::new(tx, cy - 31.0, tw, 62.0).fill("yellow").rx(31.0));
            sl.text("tag", w::SPECIAL, Tx::l(tx + 26.0, cy).cy());
        }
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (x2, top, bottom) = (sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    let mut pills = vec![(d.month.clone(), "pink", "pill_p")];
    if let Some((from, to)) = d.week {
        pills.push((format!("{} {}\u{2013}{}", w::THIS_WEEK, from, to), "yellow", "pill_t"));
    }
    page(sl, "s02", &d.kicker(w::BIRTHDAYS), Some(&d.title), &pills);
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let sparse = n as f32 <= k("sparse_names");
    let mx = 72.0;
    let _ = x2;
    let cw = if sparse { k("sparse_w") } else { W - 2.0 * mx };
    let colw = (cw - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let name_w = colw - k("day_w") - k("name_gap") - k("name_pad") - k("hl_over") / 2.0;
    let list: Vec<&str> = names.iter().map(|x| x.name.as_str()).collect();
    let size = largest_name_size(sl, &list, "name_hl", name_w, if sparse { k("sparse_size_max") } else { k("size_max") }, k("size_floor")).ok_or(Split)?;
    let pitch = (avail / per as f32).min(if sparse { 130.0 } else { k("pitch_max") });
    let y0 = if sparse { top + (avail - per as f32 * pitch) / 2.0 } else { top };
    for c in 0..cols {
        let idx: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).collect();
        if let Some(first) = idx.first() {
            let cx = mx + c as f32 * (colw + k("col_gap"));
            let at = y0 + (*first - c * per) as f32 * pitch;
            neon(sl, cx - k("hl_over"), at + 4.0, colw + 2.0 * k("hl_over") - 8.0, idx.len() as f32 * pitch - 8.0, 24.0, "yellow", 5.0, 0.0);
        }
    }
    let cap = sl.cap(&sl.face_of("name"), size);
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let cy = y0 + r as f32 * pitch + pitch / 2.0;
        let by = cy + cap / 2.0;
        let last = r == per - 1 || i + 1 == n;
        if !nm.hl && !last && !names.get(i + 1).is_some_and(|next| next.hl) {
            line(sl, cx, cy + pitch / 2.0, colw);
        }
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &format!("{:02}", nm.day), Tx::r(cx + k("day_w"), by));
        sl.text(role, &text, Tx::l(cx + k("day_w") + k("name_gap"), by).size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    let pills = d.date_long.clone().map(|date| vec![(date, "yellow", "pill_t")]).unwrap_or_default();
    page(sl, "s03", &d.kicker(w::KAS), Some(&d.title), &pills);
    let avail = bottom - top - k("total_h") - k("total_gap");
    let rp_w = sl.measure("rp", "Rp", None, None);
    let col_at = |z: f32| rp_w + k("rp_gap") + funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let lab_at = |z: f32| x2 - mx - k("bullet") - k("bullet_gap") - col_at(z) - 40.0;
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, lab_at, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let (amount_col, label_w) = (col_at(size), lab_at(size));
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let cap = sl.cap(&sl.face_of("fund"), size);
    let colours: Vec<String> = sl.d["grid"]["fund_colours"].as_array().map(|list| list.iter().filter_map(|c| c.as_str().map(str::to_string)).collect()).unwrap_or_default();
    for (i, f) in funds.iter().enumerate() {
        let colour = colours.get(i % colours.len().max(1)).cloned().unwrap_or_else(|| "pink".to_string());
        let cy = top + i as f32 * pitch + pitch / 2.0;
        let by = cy + cap / 2.0;
        sl.push(R::new(mx, cy - k("bullet") / 2.0, k("bullet"), k("bullet")).fill(&colour).rx(6.0));
        put_lines(sl, "fund", &cells[i], mx + k("bullet") + k("bullet_gap"), cy, size, label_w);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, by).size(size).fill(&colour));
        sl.text("rp", "Rp", Tx::l(x2 - amount_col, by));
        if i + 1 < funds.len() {
            line(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx);
        }
    }
    let ty = bottom - k("total_h");
    neon(sl, mx, ty, x2 - mx, k("total_h"), k("total_h") / 2.0, "pink", 6.0, 0.0);
    sl.text("total_label", w::TOTAL, Tx::l(mx + 56.0, ty + k("total_h") / 2.0).cy());
    let tv = fit::digits(d.total());
    let tw = sl.measure("total", &tv, None, None);
    sl.text("total", &tv, Tx::r(x2 - 56.0, ty + k("total_h") / 2.0).cy().maxw(x2 - mx - 560.0));
    sl.text("total_rp", "Rp", Tx::r(x2 - 56.0 - tw - 16.0, ty + k("total_h") / 2.0).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s04");
    page(sl, "s04", &d.church_kicker(""), None, &[]);
    let a = &d.account;
    let wide = if have.qris { k("left_w") } else { W - 2.0 * mx };
    let nsz = largest_step(k("number_max"), k("number_floor"), 2.0, |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let note_size = sl.ty_size("note");
    let lines = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let ncap = sl.cap(&sl.face_of("number"), nsz);
    let stack = 58.0 + 34.0 + 70.0 + 36.0 + ncap + 44.0 + 30.0 + 62.0 + 24.0 + lines.len() as f32 * nlh;
    let (py, bottom) = (sl.g("pill_y"), sl.g("body_bottom"));
    let top = py + 20.0 + (bottom - py - 20.0 - stack) / 2.0;
    let pw = sl.measure("pill_p", &d.title, None, None) + 60.0;
    neon(sl, mx, top, pw, 58.0, 29.0, "pink", 4.0, 0.0);
    sl.text("pill_p", &d.title, Tx::l(mx + 30.0, top + 29.0).cy());
    let y = top + 58.0 + 34.0;
    sl.text("bank", &a.bank, Tx::l(mx, y + 58.0).maxw(wide));
    let nb = y + 58.0 + 40.0 + ncap;
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    let y = nb + 44.0;
    sl.text("holder_l", w::HOLDER_LABEL, Tx::l(mx, y + 16.0).cy());
    let hsz = largest(sl.ty_size("holder"), 40.0, |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(40.0);
    sl.text("holder", &a.holder, Tx::l(mx, y + 16.0 + 62.0).size(hsz).maxw(wide));
    let y = y + 16.0 + 62.0 + 40.0;
    for (i, text) in lines.iter().enumerate() {
        sl.text("note", text, Tx::l(mx, y + (i as f32 + 0.8) * nlh).maxw(wide));
    }
    if have.qris {
        let card = k("card");
        let cy0 = (bottom + py) / 2.0 - card / 2.0 + 20.0;
        neon(sl, k("card_x") + k("frame_off"), cy0 + k("frame_off"), card, card, 36.0, "pink", 6.0, 0.0);
        sl.image(Pic::Qris, k("card_x"), cy0, card, card, 36.0, true, Pos::Mid);
        sl.text("caption", w::QRIS_CAPTION, Tx::c(k("card_x") + card / 2.0 + k("frame_off") / 2.0, cy0 + card + k("frame_off") + 56.0).maxw(card + 60.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let k = sl.sec("s05");
    page(sl, "s05", &d.kicker(w::VERSE_KICKER), None, &[]);
    let rw = sl.measure("ref", &d.reference, None, None) + 72.0;
    neon(sl, k("x"), 152.0, rw, 76.0, 38.0, "pink", 5.0, 0.0);
    sl.text("ref", &d.reference, Tx::l(k("x") + 36.0, 152.0 + 38.0).cy());
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: k("x"), top: k("top"), w: k("w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    sl.push(R::new(k("bar_x"), 300.0, 6.0, 420.0).fill("pink").rx(3.0));
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, bottom) = (sl.g("mx"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, if have.photo { "s06" } else { "s06np" }, &d.kicker(d.date_long.as_deref().unwrap_or("")), None, &[]);
    let wide = if have.photo { k("text_w") } else { k("nopic_w") };
    let face = sl.face_of("n_title");
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.36).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let body_h = body.iter().map(Vec::len).sum::<usize>() as f32 * blh + body.len().saturating_sub(1) as f32 * 22.0;
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, 0.0);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = largest_step(k("title_max"), k("title_floor"), 2.0, |z| wrap_at(z).len() <= 3 && wrap_at(z).len() as f32 * z * k("title_lh") + 58.0 + 36.0 + 40.0 + body_h <= bottom - 150.0).ok_or(Split)?;
    let lines = wrap_at(z);
    let block = 58.0 + 36.0 + lines.len() as f32 * z * k("title_lh") + 44.0 + body_h;
    let top = (H - block) / 2.0;
    let pw = sl.measure("pill_p", w::NOTICE_KICKER, None, None) + 60.0;
    neon(sl, mx, top, pw, 58.0, 29.0, "pink", 4.0, 0.0);
    sl.text("pill_p", w::NOTICE_KICKER, Tx::l(mx + 30.0, top + 29.0).cy());
    let mut y = top + 58.0 + 36.0;
    let cap = sl.cap(&face, z);
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, y + (i as f32 + 0.5) * z * k("title_lh") + cap / 2.0).size(z).maxw(wide));
    }
    y += lines.len() as f32 * z * k("title_lh") + 44.0;
    for (pi, para) in body.iter().enumerate() {
        for text in para {
            sl.text(if pi == 0 { "n_body_b" } else { "n_body" }, text, Tx::l(mx, y + blh * 0.78).maxw(wide));
            y += blh;
        }
        y += 22.0;
    }
    if have.photo {
        neon(sl, k("photo_x") + k("frame_off"), k("photo_y") + k("frame_off"), k("photo_w"), k("photo_h"), 30.0, "pink", 6.0, 0.0);
        sl.image(Pic::Photo, k("photo_x"), k("photo_y"), k("photo_w"), k("photo_h"), 30.0, false, Pos::Max);
    }
    Ok(())
}
