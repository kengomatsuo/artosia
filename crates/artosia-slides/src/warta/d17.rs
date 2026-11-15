//! Direction 17, Hangat: one warm hue in tone-on-tone steps, soft cards, a plate over the foot of the offering card and faint arches behind the header.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have, Name};
use super::fit;
use super::{Fits, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_step, name_in, paragraph, put_lines, verse};

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

/// The ground, the arches down to `arches` when it has one, and the kicker line with `right` at its right.
fn page(sl: &mut Sheet, d: &Data, right: &str, arches: Option<f32>) {
    let (mx, x2, cy) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_cy"));
    sl.push(R::new(0.0, 0.0, W, 1080.0).fill("ground"));
    if let Some(floor) = arches {
        let (sw, op) = (sl.s("arch", "sw"), sl.s("arch", "op"));
        for row in sl.list("arch", "rows") {
            sl.push(R::new(row[0], row[1], row[2], row[3].min(floor - row[1])).border(sw, "ink").bop(op).arch());
        }
    }
    let taken = if right.is_empty() { 0.0 } else { sl.text("kicker", right, Tx::r(x2, cy).cy().maxw((x2 - mx) / 2.0)).w + 40.0 };
    let left = sl.text("kicker", &d.church_kicker(""), Tx::l(mx, cy).cy().maxw(x2 - mx - taken - sl.mark_reserve()));
    sl.mark_after(left.x + left.w);
}

/// The title with `sub` in italic after it, when it fits the page.
fn head(sl: &mut Sheet, title: &str, sub: &str) {
    let (mx, x2, base) = (sl.g("mx"), sl.g("x2"), sl.g("title_base"));
    let t = sl.text("title", title, Tx::l(mx, base).maxw(x2 - mx));
    let at = t.x + t.w + sl.g("sub_gap");
    if !sub.is_empty() && at + sl.measure("sub", sub, None, None) <= x2 {
        sl.text("sub", sub, Tx::l(at, base));
    }
}

/// The part of a long card, muted at the foot of the slide.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::l(sl.g("mx"), sl.g("foot_cy")).cy());
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, y0, gutter, rx) = (sl.g("mx"), sl.g("x2"), sl.g("card_y"), sl.g("gutter"), sl.g("card_rx"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let floor = sl.g("card_y");
    page(sl, d, "", Some(floor));
    head(sl, &d.title, &d.week_short);
    // The rows of each day, one card a day.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if rows[i - 1].day == r.day => g.1 = i + 1,
            _ => groups.push((i, i + 1)),
        }
    }
    let count = groups.len().max(1);
    let (cols, lines) = (count.min(2), count.div_ceil(count.min(2)));
    let card_w = (x2 - mx - gutter * (cols as f32 - 1.0)) / cols as f32;
    let card_h = (sl.g("card_h") - gutter * (lines as f32 - 1.0)) / lines as f32;
    let most = groups.iter().map(|g| g.1 - g.0).max().unwrap_or(1) as f32;
    let room = card_h - 2.0 * k("pad_v") - k("head_h");
    let tr = sl.ty_size("time") / sl.ty_size("event");
    let times: Vec<&str> = rows.iter().map(|r| r.time.as_str()).collect();
    let time_w = |sl: &Sheet, z: f32| times.iter().map(|t| sl.measure("time_hl", t, Some(z * tr), None)).fold(0.0, f32::max);
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let ev_w = |z: f32| card_w - 2.0 * k("text_pad") - time_w(sl, z) - k("time_gap");
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, ev_w, k("size_max"), k("size_floor"), |z, e| most * (k("ratio") * z + e) <= room).ok_or(Split)?;
    let pitch = (room / most).min(k("ratio_max") * size + extra);
    let used = 2.0 * k("pad_v") + k("head_h") + most * pitch;
    let (tw, ew) = (time_w(sl, size), ev_w(size));
    for (g, &(from, to)) in groups.iter().enumerate() {
        let (x, y) = (mx + (g % cols) as f32 * (card_w + gutter), y0 + (g / cols) as f32 * (used + gutter));
        let tx = x + k("text_pad");
        sl.push(R::new(x, y, card_w, used).fill("card").rx(rx));
        let hy = y + k("pad_v") + k("head_h") / 2.0;
        let mut at = tx;
        if rows[from].date > 0 {
            at += sl.text("day", &rows[from].date.to_string(), Tx::l(tx, hy).cy()).w + k("head_gap");
        }
        sl.text("day_name", &rows[from].day, Tx::l(at, hy).cy().maxw(x + card_w - at - k("text_pad")));
        let top = y + k("pad_v") + k("head_h");
        for i in from..to {
            let cy = top + ((i - from) as f32 + 0.5) * pitch;
            let hl = rows[i].special;
            if hl {
                sl.push(R::new(x + k("band_in"), top + (i - from) as f32 * pitch + 3.0, card_w - 2.0 * k("band_in"), pitch - 6.0).fill("accent").rx(k("band_rx")));
            }
            sl.text(if hl { "time_hl" } else { "time" }, &rows[i].time, Tx::l(tx, cy).cy().size(size * tr).maxw(tw));
            put_lines(sl, if hl { "event_hl" } else { "event" }, &cells[i], tx + tw + k("time_gap"), cy, size, ew);
        }
    }
    Ok(())
}

/// The full name of a name's month, else the page's own heading.
fn month_of(d: &Data, name: &Name) -> String {
    (1..=12u8).find(|m| !name.month.is_empty() && d.w.month_short(*m) == name.month).map_or_else(|| d.month.clone(), |m| d.w.month(m).to_string())
}

/// What a card's row holds: its month heading, or the name at an index.
enum Cell {
    Head(String),
    Name(usize),
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, y0, gutter, rx, pad) = (sl.g("mx"), sl.g("x2"), sl.g("card_y"), sl.g("gutter"), sl.g("card_rx"), sl.g("card_pad"));
    let k = sl.sec("s02");
    let names = &d.names;
    let floor = sl.g("card_y");
    page(sl, d, "", Some(floor));
    head(sl, &d.title, &d.month);
    if d.highlighted() {
        let w = sl.measure("tag", &d.week_label, None, None) + 2.0 * k("pill_pad");
        sl.push(R::new(x2 - w, k("pill_cy") - k("pill_h") / 2.0, w, k("pill_h")).fill("accent").pill());
        sl.text("tag", &d.week_label, Tx::r(x2 - k("pill_pad"), k("pill_cy")).cy());
    }
    let cols = names.len().div_ceil(sl.g("per") as usize).clamp(1, 3);
    let per = names.len().div_ceil(cols).max(1);
    let cw = (x2 - mx - gutter * (cols as f32 - 1.0)) / cols as f32;
    // Each column names its month once, again where the month turns.
    let columns: Vec<Vec<Cell>> = (0..cols)
        .map(|c| {
            let (mut items, mut last) = (Vec::new(), None);
            for i in c * per..((c + 1) * per).min(names.len()) {
                let month = month_of(d, &names[i]);
                if last.as_ref() != Some(&month) {
                    if !month.is_empty() {
                        items.push(Cell::Head(month.clone()));
                    }
                    last = Some(month);
                }
                items.push(Cell::Name(i));
            }
            items
        })
        .collect();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let avail = sl.g("card_h") - 2.0 * k("pad_v");
    let day_w = |sl: &Sheet, z: f32| names.iter().map(|x| sl.measure("bday_day_hl", &format!("{:02}", x.day), Some(z), None)).fold(sl.g("day_w"), f32::max) + k("day_gap");
    let name_w = |sl: &Sheet, z: f32| cw - 2.0 * pad - day_w(sl, z);
    let fits = |z: f32| z * k("ratio_min") <= avail / rows as f32 && names.iter().all(|nm| fit::ladder(&nm.name, name_w(sl, z), &|s| sl.measure(if nm.hl { "name_hl" } else { "name" }, s, Some(z), None)).is_some());
    let size = sl.must(largest(grown_size(k("size_max"), k("size_big"), avail, rows, 1.6), k("size_floor"), fits), k("size_floor"))?;
    let (nx, nw) = (day_w(sl, size), name_w(sl, size));
    let pitch = (avail / rows as f32).min(k("ratio_max") * size);
    let (card_h, top) = (rows as f32 * pitch + 2.0 * k("pad_v"), y0 + k("pad_v"));
    for (c, cells) in columns.iter().enumerate() {
        let x = mx + c as f32 * (cw + gutter);
        sl.push(R::new(x, y0, cw, card_h).fill("card").rx(rx));
        // One capsule across each run of highlighted names, drawn first so the names sit on it.
        let mut j = 0;
        while j < cells.len() {
            let start = j;
            while j < cells.len() && matches!(cells[j], Cell::Name(i) if names[i].hl) {
                j += 1;
            }
            if j > start {
                sl.push(R::new(x + k("band_in"), top + start as f32 * pitch + 3.0, cw - 2.0 * k("band_in"), (j - start) as f32 * pitch - 6.0).fill("accent").rx(k("band_rx")));
            }
            j += usize::from(j == start);
        }
        for (j, cell) in cells.iter().enumerate() {
            let cy = top + (j as f32 + 0.5) * pitch;
            match cell {
                Cell::Head(month) => {
                    sl.text("sub", month, Tx::l(x + pad, cy).cy().size(k("head_size")).maxw(cw - 2.0 * pad));
                }
                Cell::Name(i) => {
                    let nm = &names[*i];
                    let (role, day_role) = if nm.hl { ("name_hl", "bday_day_hl") } else { ("name", "bday_day") };
                    sl.text(day_role, &format!("{:02}", nm.day), Tx::l(x + pad, cy).cy().size(size));
                    let (text, _) = name_in(sl, &nm.name, role, nw, size);
                    sl.text(role, &text, Tx::l(x + pad + nx, cy).cy().size(size).maxw(nw));
                }
            }
        }
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, y0, rx, pad) = (sl.g("mx"), sl.g("x2"), sl.g("card_y"), sl.g("card_rx"), sl.g("card_pad"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    let floor = sl.g("card_y");
    page(sl, d, "", Some(floor));
    head(sl, &d.title, d.date_long.as_deref().unwrap_or(""));
    let (cw, avail) = (x2 - mx, sl.g("card_h3") - 2.0 * k("pad_v"));
    let rp_w = sl.measure("rp", "Rp", None, None) + k("rp_gap");
    let digits_at = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let label_at = |z: f32| cw - 2.0 * pad - digits_at(sl, z) - rp_w - k("lab_gap");
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, label_at, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let pitch = (avail / n as f32).min(k("ratio_max") * size + extra);
    let card_h = n as f32 * pitch + 2.0 * k("pad_v");
    let label_w = label_at(size);
    sl.push(R::new(mx, y0, cw, card_h).fill("card").rx(rx));
    let top = y0 + k("pad_v");
    for (i, f) in funds.iter().enumerate() {
        let cy = top + (i as f32 + 0.5) * pitch;
        put_lines(sl, "fund", &cells[i], mx + pad, cy, size, label_w);
        let amount = sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2 - pad, cy).cy().size(size));
        sl.text("rp", "Rp", Tx::r(amount.x - k("rp_gap"), cy).cy());
        if i + 1 < n {
            sl.push(R::new(mx + pad, top + (i as f32 + 1.0) * pitch - k("rule_w") / 2.0, cw - 2.0 * pad, k("rule_w")).fill("line"));
        }
    }
    // The plate overlaps the card's foot by `plate_over`, below the last row.
    let (px, py, ph) = (mx + k("plate_in"), y0 + card_h - k("plate_over"), sl.g("plate_h"));
    sl.push(R::new(px, py, x2 - px, ph).fill("plate").rx(rx));
    let cy = py + ph / 2.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(px + k("plate_pad"), cy).cy());
    let rp = sl.measure("total_rp", "Rp", None, None) + k("total_rp_gap");
    let total = sl.text("total", &fit::digits(d.total()), Tx::r(x2 - k("plate_pad"), cy).cy().maxw(x2 - px - 2.0 * k("plate_pad") - label.w - 48.0 - rp));
    sl.text("total_rp", "Rp", Tx::r(total.x - k("total_rp_gap"), cy).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, y0, rx) = (sl.g("mx"), sl.g("x2"), sl.g("card_y"), sl.g("card_rx"));
    let k = sl.sec("s04");
    let floor = sl.g("card_y");
    page(sl, d, "", Some(floor));
    head(sl, &d.title, "");
    let plate = k("qr") + 2.0 * k("plate_pad");
    let plate_x = x2 - plate;
    let card_w = if have.qris { plate_x + k("plate_over") - mx } else { x2 - mx };
    let (tx, wide) = (mx + k("card_pad"), if have.qris { plate_x - k("text_gap") - mx - k("card_pad") } else { x2 - mx - 2.0 * k("card_pad") });
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let note_face = sl.face_of("note");
    let notes: Vec<String> = a.note.lines().filter(|line| !line.trim().is_empty()).flat_map(|line| paragraph(sl, line, &note_face, sl.ty_size("note"), wide)).collect();
    let bank = y0 + k("bank_y");
    let nb = bank + k("number_gap") + sl.cap(&sl.face_of("number"), nsz);
    let cb = nb + k("cap_gap");
    let hb = cb + k("holder_gap");
    let last = hb + k("note_gap") + notes.len().saturating_sub(1) as f32 * k("note_pitch");
    // The card is as tall as its words, never shorter than `card_h`.
    let card_h = k("card_h").max(last + k("card_pad") - y0);
    sl.need(y0 + card_h <= sl.g("card_y") + sl.g("card_h"))?;
    sl.push(R::new(mx, y0, card_w, card_h).fill("card").rx(rx));
    sl.text("bank", &a.bank, Tx::l(tx, bank).maxw(wide));
    sl.text("number", &a.number, Tx::l(tx, nb).size(nsz).maxw(wide));
    sl.text("caption", &d.w.holder, Tx::l(tx, cb));
    sl.text("holder", &a.holder, Tx::l(tx, hb).size(hsz).maxw(wide));
    for (i, line) in notes.iter().enumerate() {
        sl.text("note", line, Tx::l(tx, hb + k("note_gap") + i as f32 * k("note_pitch")).maxw(wide));
    }
    if have.qris {
        // The code is dark on light, so its plate is cream whatever the colourway.
        sl.push(R::new(plate_x, k("plate_y"), plate, plate).fill("#FBF5EA").rx(rx));
        sl.image(Pic::Qris, plate_x + k("plate_pad"), k("plate_y") + k("plate_pad"), k("qr"), k("qr"), 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(plate_x + plate / 2.0, k("plate_y") + plate + k("caption_gap")).maxw(plate + 120.0));
    }
    Ok(())
}

/// The picture, a rounded rectangle at the right, drawn by the verse and the notice.
fn picture(sl: &mut Sheet, k: &dyn Fn(&str) -> f32) {
    sl.image(Pic::Photo, sl.g("x2") - k("photo_w"), k("photo_y"), k("photo_w"), k("photo_h"), k("photo_rx"), false, Pos::Mid);
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref().unwrap_or(""), (!have.photo).then_some(f32::MAX));
    sl.text("caption", &d.w.verse, Tx::l(mx, k("caption_y")).cy());
    let wide = if have.photo { k("text_w_pic") } else { k("text_w") };
    if !d.reference.is_empty() {
        sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(wide));
    }
    if have.photo {
        picture(sl, &k);
    }
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: wide, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s06");
    page(sl, d, d.date_long.as_deref().unwrap_or(""), (!have.photo).then_some(f32::MAX));
    sl.text("caption", &d.w.notice, Tx::l(mx, k("caption_y")).cy());
    let wide = if have.photo { sl.g("x2") - mx - k("photo_w") - k("photo_gap") } else { k("text_w") };
    if have.photo {
        picture(sl, &k);
    }
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, 0.0);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    // As large as three lines hold, down to the floor.
    let (max, floor) = (k("title_max"), k("title_floor"));
    let z = sl.must(largest_step(max, floor, 1.0, |z| wrap_at(z).len() <= 3), floor)?;
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
    sl.need(last <= k("bottom"))?;
    Ok(())
}
