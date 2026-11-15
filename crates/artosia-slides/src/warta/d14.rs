//! Direction 14, Cahaya: a dark gradient field with one soft orb of light, light tracked caps and light figures, the data in plain white.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have, Slot};
use super::fit;
use super::{Fits, H, LINE, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_step, name_in, paragraph, paragraphs, put_lines, repeats, verse};

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

/// The part of a long card, muted at the foot of the slide.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::l(sl.g("mx"), sl.g("foot_cy")).cy());
}

/// The gradient ground, the orb of two radial glows, the kicker and `tag` in a light pill at the right;
/// returns the width left for the title.
fn page(sl: &mut Sheet, d: &Data, tag: Option<&str>) -> f32 {
    let (mx, x2, ky) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_cy"));
    sl.push(R::new(0.0, 0.0, W, H).fill("grad:ground"));
    for name in ["orb1", "orb2"] {
        // Each glow is drawn on its own circle's box, clipped to the sheet.
        let (cx, cy, r) = (sl.glow(name, "cx"), sl.glow(name, "cy"), sl.glow(name, "r"));
        let (x0, y0, x1, y1) = ((cx - r).max(0.0), (cy - r).max(0.0), (cx + r).min(W), (cy + r).min(H));
        sl.push(R::new(x0, y0, x1 - x0, y1 - y0).fill(&format!("grad:{name}")));
    }
    sl.text("kicker", &d.church_kicker(""), Tx::l(mx, ky).cy().maxw(x2 - mx));
    let mut room = x2 - mx;
    if let Some(tag) = tag.filter(|tag| !tag.is_empty()) {
        let (pad, h, cy) = (sl.g("pill_pad"), sl.g("pill_h"), sl.g("pill_cy"));
        let w = (sl.measure("tag", tag, None, None) + 2.0 * pad).min((x2 - mx) / 2.0);
        sl.push(R::new(x2 - w, cy - h / 2.0, w, h).fill("pill").pill());
        sl.text("tag", tag, Tx::r(x2 - pad, cy).cy().maxw(w - 2.0 * pad));
        room = x2 - w - 48.0 - mx;
    }
    room
}

/// The title in light caps and `sub` under it, both within `room`.
fn head(sl: &mut Sheet, title: &str, sub: Option<&str>, room: f32) {
    let (mx, base, sub_cy) = (sl.g("mx"), sl.g("title_base"), sl.g("sub_cy"));
    sl.text("title", title, Tx::l(mx, base).maxw(room));
    if let Some(sub) = sub.filter(|sub| !sub.is_empty() && !repeats(sub, title)) {
        sl.text("sub", sub, Tx::l(mx, sub_cy).cy().maxw(room));
    }
}

/// The highlight: a white capsule at 12% with an accent bar at its left.
fn capsule(sl: &mut Sheet, x: f32, y: f32, w: f32, h: f32) {
    let (rx, bar_in, bar_pad, bar_w) = (sl.g("band_rx"), sl.g("bar_in"), sl.g("bar_pad"), sl.g("bar_w"));
    sl.push(R::new(x, y, w, h).fill("glass").op(0.12).rx(rx));
    sl.push(R::new(x + bar_in, y + bar_pad, bar_w, h - 2.0 * bar_pad).fill("accent").rx(bar_w / 2.0));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, gutter, over) = (sl.g("mx"), sl.g("x2"), sl.g("gutter"), sl.g("band_over"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let room = page(sl, d, Some(d.week_label.as_str()));
    head(sl, &d.title, None, room);
    // The rows of each day, then the days in two columns of near-equal height.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if rows[i - 1].day == r.day && rows[i - 1].date == r.date => g.1 = i + 1,
            _ => groups.push((i, i + 1)),
        }
    }
    let weight = |gs: &[(usize, usize)]| gs.iter().map(|g| g.1 - g.0 + 1).sum::<usize>();
    let cut = (1..=groups.len()).min_by_key(|s| weight(&groups[..*s]).max(weight(&groups[*s..]))).unwrap_or(0);
    let columns: Vec<&[(usize, usize)]> = [&groups[..cut], &groups[cut..]].into_iter().filter(|gs| !gs.is_empty()).collect();
    let two_w = ((x2 - mx) - gutter) / 2.0;
    let colw = if columns.len() > 1 { two_w } else { x2 - mx };
    let time_w = two_w - k("event_w");
    let ev_w = colw - time_w;
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let hi = if rows.len() as f32 <= k("sparse_rows") { k("sparse_size_max") } else { k("size_max") };
    let avail = k("bot") - k("top");
    // A column is its rows at the ratio, its day headings and a second line for each event that needs one.
    let height = |sl: &Sheet, z: f32, e: f32| {
        columns
            .iter()
            .map(|gs| {
                let count: usize = gs.iter().map(|g| g.1 - g.0).sum();
                let two = if e > 0.0 { gs.iter().flat_map(|g| g.0..g.1).filter(|i| sl.measure("event", events[*i], Some(z), None) > ev_w).count() } else { 0 };
                count as f32 * k("ratio") * z + gs.len() as f32 * k("head_h") + (gs.len() - 1) as f32 * k("day_gap") + e * two as f32
            })
            .fold(0.0, f32::max)
    };
    let (size, cells, _) = fit_cells(sl, "event", &events, |_| ev_w, hi, k("size_floor"), |z, e| height(sl, z, e) <= avail).ok_or(Split)?;
    for (c, gs) in columns.iter().enumerate() {
        let x0 = mx + c as f32 * (colw + gutter);
        let mut y = k("top");
        for (at, &(from, to)) in gs.iter().enumerate() {
            if at > 0 {
                y += k("day_gap");
            }
            let head_text = if rows[from].date > 0 { format!("{} {}", rows[from].day, rows[from].date) } else { rows[from].day.clone() };
            sl.text("day", &head_text, Tx::l(x0 + k("time_in"), y + k("head_h") / 2.0).cy().maxw(colw));
            y += k("head_h");
            for i in from..to {
                let rh = k("ratio") * size + if cells[i].len() > 1 { LINE * size } else { 0.0 };
                if rows[i].special {
                    capsule(sl, x0 - over, y + 3.0, colw + 2.0 * over, rh - 6.0);
                }
                sl.text("time", &rows[i].time, Tx::l(x0 + k("time_in"), y + rh / 2.0).cy().maxw(time_w - k("time_in") - 8.0));
                put_lines(sl, "event", &cells[i], x0 + time_w, y + rh / 2.0, size, ev_w);
                y += rh;
            }
        }
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, gutter, over, pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bot"), sl.g("gutter"), sl.g("band_over"), sl.g("row_pad"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    let room = page(sl, d, d.highlighted().then_some(d.week_label.as_str()));
    head(sl, &d.title, Some(&d.month), room);
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let columns = d.month_runs(n.div_ceil(cols), cols);
    let rows = columns.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let (colw, avail) = (((x2 - mx) - gutter * (cols as f32 - 1.0)) / cols as f32, bottom - top);
    let (day_w0, day_in) = (sl.g("day_w"), sl.g("day_in"));
    let day_w = |sl: &Sheet, z: f32| day_w0.max(sl.measure("bday_day", "00", Some(z), None));
    let room_at = |sl: &Sheet, z: f32| colw - day_in - day_w(sl, z) - 2.0 * pad;
    let role_of = |i: usize| if names[i].hl { "name_hl" } else { "name" };
    let shortened = |sl: &Sheet, z: f32| (0..names.len()).filter(|i| name_in(sl, &names[*i].name, role_of(*i), room_at(sl, z), z).1).count();
    // The largest size that shortens no more names than the smallest does.
    let at_floor = shortened(sl, k("size_floor"));
    let hi = grown_size(k("size_max"), k("sparse_size_max"), avail, rows, k("ratio"));
    let size = largest(hi, k("size_floor"), |z| shortened(sl, z) <= at_floor).ok_or(Split)?;
    let pitch = (avail / rows as f32).min(k("pitch_max") * size);
    let (dw, name_w) = (day_w(sl, size), room_at(sl, size));
    let day_z = size.min(sl.ty_size("bday_day"));
    for (c, column) in columns.iter().enumerate() {
        let cx = mx + c as f32 * (colw + gutter);
        // A run of highlighted names is one capsule.
        let mut at = 0;
        while at < column.len() {
            if !matches!(column[at], Slot::Name(i) if names[i].hl) {
                at += 1;
                continue;
            }
            let from = at;
            while at < column.len() && matches!(column[at], Slot::Name(i) if names[i].hl) {
                at += 1;
            }
            capsule(sl, cx - over, top + from as f32 * pitch + 3.0, colw + 2.0 * over, (at - from) as f32 * pitch - 6.0);
        }
        for (j, slot) in column.iter().enumerate() {
            let cy = top + (j as f32 + 0.5) * pitch;
            match slot {
                Slot::Head(month) => {
                    sl.text("caption", month, Tx::l(cx + day_in, cy).cy().maxw(colw - day_in));
                }
                Slot::Name(i) => {
                    let nm = &names[*i];
                    sl.text("bday_day", &format!("{:02}", nm.day), Tx::l(cx + day_in, cy).cy().size(day_z));
                    let (text, _) = name_in(sl, &nm.name, role_of(*i), name_w, size);
                    sl.text(role_of(*i), &text, Tx::l(cx + day_in + dw + pad, cy).cy().size(size).maxw(name_w));
                }
            }
        }
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    let room = page(sl, d, d.date_long.as_deref());
    head(sl, &d.title, Some(&d.w.offerings), room);
    let (top, list_r) = (k("rows_top"), k("list_r"));
    let avail = k("rows_bot") - top;
    let z = largest(k("size_max"), k("size_floor"), |z| n as f32 * k("ratio") * z <= avail).ok_or(Split)?;
    let rp_z = (sl.ty_size("rp") * z / sl.ty_size("amount")).round();
    let digits: Vec<String> = funds.iter().map(|f| fit::digits(f.rupiah)).collect();
    let widths: Vec<f32> = digits.iter().map(|s| sl.measure("amount", s, Some(z), None)).collect();
    let rp_w = sl.measure("rp", "Rp", Some(rp_z), None);
    let col = widths.iter().copied().fold(0.0, f32::max) + k("rp_gap") + rp_w;
    let label_w = list_r - mx - col - k("lab_gap");
    let pitch = (avail / n as f32).min(k("ratio_max") * z);
    for (i, f) in funds.iter().enumerate() {
        let cy = top + (i as f32 + 0.5) * pitch;
        // A label that is too wide is set smaller, down to the floor, then on two lines.
        let face = sl.face_of("fund");
        let one = largest(z, k("label_floor").min(z), |s| sl.measure("fund", &f.label, Some(s), None) <= label_w);
        let (size, lines) = match one {
            Some(s) => (s, vec![f.label.clone()]),
            None => {
                let s = k("label_floor").min(z);
                let lines = paragraph(sl, &f.label, &face, s, label_w);
                sl.need(lines.len() <= 2)?;
                (s, lines)
            }
        };
        put_lines(sl, "fund", &lines, mx, cy, size, label_w);
        sl.text("amount", &digits[i], Tx::r(list_r, cy).cy().size(z));
        sl.text("rp", "Rp", Tx::r(list_r - widths[i] - k("rp_gap"), cy).cy().size(rp_z));
        if i + 1 < funds.len() {
            let h = sl.g("rule_w");
            sl.push(R::new(mx, top + (i + 1) as f32 * pitch - h / 2.0, list_r - mx, h).fill("mute").op(sl.g("rule_op")));
        }
    }
    // The total is a very light figure at the lower right, solved to the room it has.
    let total = fit::digits(d.total());
    let rp_big = k("total_rp");
    let rp_big_w = sl.measure("total", "Rp", Some(rp_big), None) + k("rp_to_fig");
    let fig = sl.must(largest(k("total_max"), k("total_min"), |z| sl.measure("total", &total, Some(z), None) <= x2 - mx - rp_big_w), k("total_min"))?;
    sl.text("total_label", &d.w.total_gifts, Tx::r(x2, k("tot_label_cy")).cy().maxw(x2 - mx));
    let at = sl.text("total", &total, Tx::r(x2, k("tot_fig_cy")).cy().size(fig));
    sl.text("total", "Rp", Tx::r(at.x - k("rp_to_fig"), k("tot_fig_cy")).cy().size(rp_big));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s04");
    page(sl, d, Some(d.title.as_str()));
    let plate = k("plate");
    let wide = if have.qris { x2 - plate - k("col_gap") - mx } else { x2 - mx };
    let a = &d.account;
    let face = sl.face_of("number");
    let one_line = |z: f32| sl.measure("number", &a.number, Some(z), None) <= wide;
    let two_lines = |z: f32| {
        let lines = paragraph(sl, &a.number, &face, z, wide);
        lines.len() <= 2 && lines.iter().all(|line| sl.measure("number", line, Some(z), None) <= wide)
    };
    // One line when the digits hold it above the floor, else two balanced lines as large as they go.
    let nsz = largest(k("number_max"), k("number_floor"), one_line).or_else(|| largest(k("number_max"), k("number_floor"), two_lines)).unwrap_or(k("number_floor"));
    let nlines = if one_line(nsz) { vec![a.number.clone()] } else { paragraph(sl, &a.number, &face, nsz, wide) };
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let (bcap, ncap, hcap) = (sl.cap(&sl.face_of("bank"), sl.ty_size("bank")), sl.cap(&face, nsz), sl.cap(&sl.face_of("holder"), hsz));
    let (note_z, nlh) = (sl.ty_size("note"), (sl.ty_size("note") * 1.4).round());
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_z, wide) };
    let lh = k("lh") * nsz;
    let num_h = ncap + (nlines.len() as f32 - 1.0) * lh;
    let stack = bcap + 56.0 + num_h + 64.0 + nlh + hcap + if notes.is_empty() { 0.0 } else { 36.0 + notes.len() as f32 * nlh };
    let top = k("top").max((k("top") + k("bot") - stack) / 2.0);
    sl.need(top + stack <= k("bot"))?;
    sl.text("bank", &a.bank, Tx::l(mx, top + bcap).maxw(wide));
    let n0 = top + bcap + 56.0 + ncap;
    for (i, line) in nlines.iter().enumerate() {
        sl.text("number", line, Tx::l(mx, n0 + i as f32 * lh).size(nsz).maxw(wide));
    }
    let nb = n0 + (nlines.len() as f32 - 1.0) * lh;
    sl.text("note", &d.w.holder, Tx::l(mx, nb + 64.0 + nlh * 0.8).maxw(wide));
    let hb = nb + 64.0 + nlh + hcap;
    sl.text("holder", &a.holder, Tx::l(mx, hb).size(hsz).maxw(wide));
    let mut yy = hb + 36.0 + nlh * 0.8;
    for line in &notes {
        sl.text("note", line, Tx::l(mx, yy).maxw(wide));
        yy += nlh;
    }
    if have.qris {
        // The code is dark on light, so it sits on a white card over the gradient.
        let (px, py) = (x2 - plate, k("plate_y"));
        sl.push(R::new(px, py, plate, plate).fill("#FFFFFF").rx(k("plate_rx")));
        sl.image(Pic::Qris, px + k("pad"), py + k("pad"), plate - 2.0 * k("pad"), plate - 2.0 * k("pad"), 0.0, true, Pos::Mid);
        sl.text("caption", d.w.scan.as_str(), Tx::c(px + plate / 2.0, py + plate + 44.0).cy().maxw(plate + 120.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref());
    let w = if have.photo { k("text_w_pic") } else { k("text_w") };
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_cy")).cy().maxw(w));
    if have.photo {
        let (px, top) = (x2 - k("pic_w"), k("top"));
        sl.image(Pic::Photo, px, top, k("pic_w"), k("bottom") - top, k("pic_rx"), false, Pos::Mid);
    }
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bot"));
    let k = sl.sec("s06");
    page(sl, d, None);
    let wide = if have.photo { k("text_w_pic") } else { k("text_w") };
    if have.photo {
        // Two pictures stack below the orb; with one it is used twice, cropped from each end.
        let (px, ph) = (x2 - k("photo_w"), k("photo_h"));
        let (first, second) = if have.photo2 { (Pos::Mid, Pos::Mid) } else { (Pos::Min, Pos::Max) };
        sl.image(Pic::Photo, px, k("photo_top"), k("photo_w"), ph, k("photo_rx"), false, first);
        sl.image(Pic::Photo2, px, k("photo_top") + ph + k("photo_gap"), k("photo_w"), ph, k("photo_rx"), false, second);
    }
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, ls * z);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("title_top") + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let mut y = k("title_top") + cap + (lines.len() as f32 - 1.0) * tl + k("body_gap");
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

#[cfg(test)]
mod tests {
    use super::super::*;
    use artosia_core::calendar::Date;

    /// The library spec's 30 birthday names, 22 November to 20 December, six of them highlighted.
    fn mock_birthdays() -> Slide {
        let rows: [(u8, u8, &str); 30] = [
            (22, 11, "Bpk. Daniel Hutapea"), (22, 11, "Ibu Marta Sihombing"), (23, 11, "Ny. Yohana R. Kapitarau Sumual"), (24, 11, "Bpk. Yohanes B. Tampubolon"), (25, 11, "Adk. Gabriel Pangaribuan"),
            (26, 11, "Ibu Ruth Simanjuntak"), (27, 11, "Ny. Elisabeth Mandagi"), (28, 11, "Bpk. Samuel Nainggolan"), (29, 11, "Ibu Hana Kristiani"), (30, 11, "Bpk. Markus T. Lumbantobing"),
            (1, 12, "Sdri. Naomi Agustina Siregar"), (1, 12, "Adk. Timotius Wijaya"), (3, 12, "Ny. Maria S. Tuwaidan Pandey"), (4, 12, "Bpk. Petrus Sinaga"), (5, 12, "Ibu Debora Panggabean"),
            (6, 12, "Bpk. Andreas Purba"), (7, 12, "Ibu Yuliana Napitupulu"), (8, 12, "Sdr. Benyamin Situmorang"), (9, 12, "Ibu Sarah Manurung"), (10, 12, "Ny. Rosalina Wenas"),
            (11, 12, "Bpk. Hendrik Lengkong"), (12, 12, "Ibu Agnes Rumbayan"), (13, 12, "Sdri. Natalia Parinussa"), (14, 12, "Bpk. Joshua Ambarita"), (15, 12, "Ibu Katarina Watuseke"),
            (16, 12, "Adk. Elia Pardede"), (17, 12, "Ny. Dorkas Simatupang"), (18, 12, "Bpk. Yakobus Hutagalung"), (19, 12, "Ibu Priscilla Matulessy"), (20, 12, "Sdr. Abraham Tumiwa"),
        ];
        let text: Vec<String> = rows.iter().map(|(day, _, name)| format!("{name}\t{day} X")).collect();
        let dates = rows.iter().map(|(day, month, _)| Date { year: 2026, month: *month, day: *day }).collect();
        let week = Some((Date { year: 2026, month: 11, day: 29 }, Date { year: 2026, month: 12, day: 5 }));
        Slide { notice: NoticeKind::Birthdays, heading: "Ulang Tahun".into(), text: text.join("\n"), dates, marked: (8..15).collect(), week, ..Slide::default() }
    }

    fn texts(sheet: &Sheet, role: &str) -> Vec<Tt> {
        sheet.items.iter().filter_map(|item| if let Item::Text(t) = item { Some(t.clone()) } else { None }).filter(|t| t.role == role).collect()
    }

    #[test]
    fn thirty_names_across_two_months_are_one_slide_with_a_heading_in_each_column() {
        let slide = mock_birthdays();
        let have = data::Have { photo: false, photo2: false, qris: false };
        let sheet = build(&slide, NoticeStyle::Glow, have, None, false).expect("30 names hold");
        let heads = texts(&sheet, "caption");
        assert_eq!(heads.iter().map(|t| t.s.as_str()).collect::<Vec<_>>(), ["NOVEMBER", "DESEMBER", "DESEMBER"]);
        let names: Vec<Tt> = texts(&sheet, "name").into_iter().chain(texts(&sheet, "name_hl")).collect();
        let original: Vec<&str> = slide.text.lines().map(|line| line.split('\t').next().unwrap_or("")).collect();
        let shortened = names.iter().filter(|t| !original.contains(&t.s.as_str())).count();
        eprintln!("size {} shortened {shortened} names {}", names[0].size, names.len());
        assert!(shortened <= 6, "{shortened} names take initials");
        assert!(names.iter().all(|t| (t.size - names[0].size).abs() < f32::EPSILON), "one size for every name");
        // The library spec's mock: row pitch 62.73, month heading baseline 365.04, first row 430.49, name at 136.
        let close = |got: f32, want: f32, tolerance: f32| assert!((got - want).abs() <= tolerance, "{got} is not {want} within {tolerance}");
        let days = texts(&sheet, "bday_day");
        close(days[1].y - days[0].y, 62.73, 2.0);
        close(heads[0].y, 365.04, 2.0);
        close(days[0].y, 430.49, 2.0);
        close(days[0].x, 78.0, 1.0);
        close(texts(&sheet, "name")[0].x, 136.0, 4.0);
        close(texts(&sheet, "kicker")[0].x, 72.0, 1.0);
        close(texts(&sheet, "title")[0].size, 96.0, 2.0);
        assert_eq!(texts(&sheet, "bday_day").len(), 30, "a row writes its day only");
        assert!(sheet.items.iter().all(|item| !matches!(item, Item::Text(t) if t.role == "bday_day" && t.s.len() > 2)));
    }

    #[test]
    fn a_week_of_eight_rows_in_four_days_is_one_slide() {
        let text = "Minggu\t07.00\tIbadah Raya I\n\t09.30\tIbadah Raya II dan Sekolah Minggu\nRabu\t18.30\tPersekutuan Doa dan Pemahaman Alkitab\n\t20.00\tLatihan Paduan Suara dan Tim Pemusik\nJumat\t17.30\tPelayanan Kunjungan Kasih ke Rumah Sakit\n\t19.00\tIbadah Doa Puasa Bersama Seluruh Jemaat\nSabtu\t09.00\tRapat Majelis dan Pengurus Komisi Anak\n\t16.00\tMalam Pujian dan Penyembahan Khusus Natal";
        let slide = Slide { notice: NoticeKind::Schedule, heading: "Jadwal Pekan Ini".into(), text: text.into(), marked: vec![7], ..Slide::default() };
        assert_eq!(paginate(&slide, NoticeStyle::Glow).len(), 1);
    }

    #[test]
    fn the_orb_is_two_radial_glows_inside_the_sheet() {
        let sheet = build(&mock_birthdays(), NoticeStyle::Glow, data::Have { photo: false, photo2: false, qris: false }, None, false).expect("holds");
        let glows: Vec<&R> = sheet.items.iter().filter_map(|item| if let Item::Rect(r) = item { Some(r) } else { None }).filter(|r| r.fill.as_deref().is_some_and(|f| f.starts_with("grad:orb"))).collect();
        assert_eq!(glows.len(), 2);
        assert!(glows.iter().all(|r| r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= W && r.y + r.h <= H));
        assert!(sheet.radial("orb1", glows[0], 1.0).is_some_and(|(_, radius, stops)| radius == 780.0 && stops.len() == 3));
    }
}
