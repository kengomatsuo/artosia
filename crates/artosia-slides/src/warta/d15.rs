//! Direction 15, Majalah: a magazine page. A double rule under the masthead, a text-serif headline, corner lines in small caps, and rows that read like a contents page.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have};
use super::fit;
use super::{Fits, LINE, Pic, Pos, R, Sheet, Split, Tx, VerseBox, fit_cells, grown_size, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The part of a long card, in the bottom-right corner where a folio belongs.
pub fn foot(sl: &mut Sheet, note: &str) {
    sl.text("foot", note, Tx::r(sl.g("x2"), sl.g("meta_bot_cy")).cy());
}

/// A thick and a thin rule, `y` the top of the thick one.
fn double_rule(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let (thick, gap, thin) = (sl.g("rule_thick"), sl.g("rule_gap"), sl.g("rule_thin"));
    sl.push(R::new(x, y, width, thick).fill("ink"));
    sl.push(R::new(x, y + thick + gap, width, thin).fill("ink"));
}

/// A hairline `y` is the top of.
fn hair(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    let height = sl.g("hair_w");
    sl.push(R::new(x, y, width, height).fill("rule"));
}

/// A hairline down a column, `x` its middle.
fn vrule(sl: &mut Sheet, x: f32, top: f32, bottom: f32) {
    let width = sl.g("vrule_w");
    sl.push(R::new(x - width / 2.0, top, width, bottom - top).fill("rule"));
}

/// The ground, the two corner lines at the top, the double rule to `x2` and the line at the bottom left.
fn chrome(sl: &mut Sheet, d: &Data, x2: f32, bottom_left: Option<&str>) {
    let (mx, top, bottom) = (sl.g("mx"), sl.g("meta_top_cy"), sl.g("meta_bot_cy"));
    sl.push(R::new(0.0, 0.0, super::W, super::H).fill("ground"));
    let left = sl.text("kicker", &d.w.warta, Tx::l(mx, top).cy());
    // A church name too long to sit beside the warta line is left out.
    sl.mark_after(left.x + left.w);
    if !d.church.is_empty() && sl.measure("kicker", &d.church, None, None) <= x2 - mx - left.w - 48.0 - sl.mark_reserve() {
        sl.text("kicker", &d.church, Tx::r(x2, top).cy());
    }
    double_rule(sl, mx, sl.g("rule_y"), x2 - mx);
    if let Some(line) = bottom_left.filter(|line| !line.is_empty()) {
        sl.text("kicker", line, Tx::l(mx, bottom).cy());
    }
}

/// The headline as large as the room beside the italic `sub` at its right lets it be.
fn headline(sl: &mut Sheet, text: &str, sub: &str, x2: f32) {
    let (mx, base) = (sl.g("mx"), sl.g("title_base"));
    let sub_w = if sub.is_empty() { 0.0 } else { sl.measure("sub", sub, None, None) + 40.0 };
    let room = x2 - mx - sub_w;
    let z = largest(sl.ty_size("title"), 64.0, |z| sl.measure("title", text, Some(z), None) <= room).unwrap_or(64.0);
    sl.text("title", text, Tx::l(mx, base).size(z).maxw(room));
    if !sub.is_empty() {
        sl.text("sub", sub, Tx::r(x2, base));
    }
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, gutter, over) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bot"), sl.g("gutter"), sl.g("band_over"));
    let (pad, k, rows) = (sl.g("row_pad"), sl.sec("s01"), &d.rows);
    chrome(sl, d, x2, None);
    headline(sl, &d.title, &d.week_label, x2);
    // Rows are grouped by day, each group a section head over its rows.
    let mut groups: Vec<usize> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match groups.last_mut() {
            Some(n) if i > 0 && r.day == rows[i - 1].day && r.date == rows[i - 1].date => *n += 1,
            _ => groups.push(1),
        }
    }
    // Two columns hold the day groups in order, split where the two weigh the most alike.
    let weight = |part: &[usize]| part.iter().map(|n| *n as f32 + 1.0).sum::<f32>();
    let cols = if groups.len() > 1 { 2 } else { 1 };
    let first = if cols == 1 { groups.len() } else { (1..groups.len()).min_by(|a, b| weight(&groups[..*a]).max(weight(&groups[*a..])).total_cmp(&weight(&groups[*b..]).max(weight(&groups[..*b])))).unwrap_or(1) };
    let parts = [&groups[..first], &groups[first..]];
    let colw = (x2 - mx - gutter * (cols as f32 - 1.0)) / cols as f32;
    let event_w = colw - k("time_w");
    let avail = bottom - top;
    let height = |part: &[usize], z: f32, extra: f32| part.iter().map(|n| k("head_h") + *n as f32 * (k("ratio") * z + extra)).sum::<f32>();
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event", &events, |_| event_w, k("size_max"), k("size_floor"), |z, e| parts.iter().all(|part| height(part, z, e) <= avail)).ok_or(Split)?;
    let (pitch, time_z) = (k("ratio") * size + extra, (size * 0.96).round());
    if cols == 2 {
        vrule(sl, mx + colw + gutter / 2.0, top, bottom);
    }
    let mut at = 0;
    for (c, part) in parts.iter().take(cols).enumerate() {
        let cx = mx + c as f32 * (colw + gutter);
        let mut y = top;
        for &n in *part {
            let lead = &rows[at];
            let date = match (lead.date, lead.month.is_empty()) {
                (0, _) => String::new(),
                (day, true) => day.to_string(),
                (day, false) => format!("{day} {}", lead.month),
            };
            let (name_w, mid) = (sl.measure("day", &lead.day, None, None), y + k("head_h") / 2.0);
            sl.text("day", &lead.day, Tx::l(cx + pad, mid).cy().maxw(colw));
            sl.text("day_date", &date, Tx::l(cx + pad + name_w + k("head_gap"), mid).cy().maxw(colw - name_w - k("head_gap")));
            hair(sl, cx, y + k("head_h") - 6.0, colw);
            y += k("head_h");
            for j in 0..n {
                let (r, cell) = (&rows[at + j], &cells[at + j]);
                let mid = y + pitch / 2.0;
                if r.special {
                    sl.push(R::new(cx - over, y + 2.0, colw + 2.0 * over, pitch - 4.0).fill("inv_bg"));
                }
                let (time, event) = if r.special { ("time_hl", "event_hl") } else { ("time", "event") };
                sl.text(time, &r.time, Tx::l(cx + pad, mid - (cell.len() as f32 - 1.0) / 2.0 * LINE * size).cy().size(time_z).maxw(k("time_w") - pad - 12.0));
                put_lines(sl, event, cell, cx + k("time_w"), mid, size, event_w);
                y += pitch;
            }
            at += n;
        }
    }
    Ok(())
}

/// A month's long name from the short one a name carries.
fn long_month(d: &Data, short: &str) -> String {
    (1..=12).find(|m| d.w.month_short(*m) == short).map_or_else(|| short.to_string(), |m| d.w.month(m).to_string())
}

/// A row of a birthday column: the month heading that opens a run of names, or a name.
enum Row {
    Head(String),
    Name(usize),
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, gutter, over, pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bot"), sl.g("gutter"), sl.g("band_over"), sl.g("row_pad"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    chrome(sl, d, x2, d.date_long.as_deref());
    headline(sl, &d.title, &d.month, x2);
    if d.highlighted() {
        sl.text("tag", &d.week_label, Tx::r(x2, sl.g("aside_cy")).cy());
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    // A column heads each month it holds once, so a row never writes its month.
    let columns: Vec<Vec<Row>> = (0..cols)
        .map(|c| {
            let mut column = Vec::new();
            let mut last = None;
            for i in c * per..((c + 1) * per).min(names.len()) {
                if d.spans && last != Some(&names[i].month) {
                    column.push(Row::Head(long_month(d, &names[i].month)));
                    last = Some(&names[i].month);
                }
                column.push(Row::Name(i));
            }
            column
        })
        .collect();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let (colw, avail) = (((x2 - mx) - gutter * (cols as f32 - 1.0)) / cols as f32, bottom - top);
    let day_w = |sl: &Sheet, z: f32| k("day_w").max(sl.measure("bday_day_hl", "00", Some(z), None));
    let room = |sl: &Sheet, z: f32| colw - pad - day_w(sl, z) - k("day_gap");
    let role_of = |i: usize| if names[i].hl { "name_hl" } else { "name" };
    let shortened = |sl: &Sheet, z: f32| (0..names.len()).filter(|i| name_in(sl, &names[*i].name, role_of(*i), room(sl, z), z).1).count();
    // The largest size that shortens no more names than the smallest does.
    let at_floor = shortened(sl, k("size_floor"));
    let hi = grown_size(k("size_max"), k("size_max") * 1.7, avail, rows, k("ratio"));
    let size = largest(hi, k("size_floor"), |z| shortened(sl, z) <= at_floor).ok_or(Split)?;
    let pitch = (avail / rows as f32).min(k("pitch_max") * size);
    let (dw, room) = (day_w(sl, size), room(sl, size));
    if cols > 1 {
        for c in 1..cols {
            vrule(sl, mx + c as f32 * (colw + gutter) - gutter / 2.0, top, bottom);
        }
    }
    for (c, column) in columns.iter().enumerate() {
        let cx = mx + c as f32 * (colw + gutter);
        // A run of highlighted names is one inverted block.
        let mut j = 0;
        while j < column.len() {
            let marked = |at: usize| matches!(column.get(at), Some(Row::Name(i)) if names[*i].hl);
            if !marked(j) {
                j += 1;
                continue;
            }
            let run = (j..column.len()).take_while(|at| marked(*at)).count();
            sl.push(R::new(cx - over, top + j as f32 * pitch + 2.0, colw + 2.0 * over, run as f32 * pitch - 4.0).fill("inv_bg"));
            j += run;
        }
        for (j, row) in column.iter().enumerate() {
            let cy = top + (j as f32 + 0.5) * pitch;
            match row {
                Row::Head(month) => {
                    sl.text("caption", month, Tx::l(cx + pad, cy).cy().fill("accent"));
                    hair(sl, cx, cy + pitch / 2.0 - 2.0, colw);
                }
                Row::Name(i) => {
                    let (name, role, day) = (&names[*i], role_of(*i), if names[*i].hl { "bday_day_hl" } else { "bday_day" });
                    let (text, _) = name_in(sl, &name.name, role, room, size);
                    sl.text(day, &format!("{:02}", name.day), Tx::l(cx + pad, cy).cy().size(size));
                    sl.text(role, &text, Tx::l(cx + pad + dw + k("day_gap"), cy).cy().size(size).maxw(room));
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
    chrome(sl, d, x2, None);
    headline(sl, &d.title, d.date_long.as_deref().unwrap_or(""), x2);
    sl.text("caption", &d.w.offerings, Tx::l(mx, k("head_cy")).cy());
    sl.text("caption", &d.w.total, Tx::r(x2, k("head_cy")).cy());
    sl.push(R::new(mx, k("head_rule_y"), x2 - mx, sl.g("rule_thin")).fill("ink"));
    let rp_w = sl.measure("rp", "Rp", None, None);
    let widest = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let label_room = |sl: &Sheet, z: f32| x2 - mx - widest(sl, z) - k("rp_gap") - rp_w - k("lab_gap");
    let avail = k("rows_bot") - k("rows_top");
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, |z| label_room(sl, z), k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let (pitch, label_w) = ((avail / n as f32).min(k("ratio_max") * size + extra), label_room(sl, size));
    for (i, f) in funds.iter().enumerate() {
        let cy = k("rows_top") + (i as f32 + 0.5) * pitch;
        let amount = fit::digits(f.rupiah);
        put_lines(sl, "fund", &cells[i], mx, cy, size, label_w);
        sl.text("amount", &amount, Tx::r(x2, cy).cy().size(size));
        sl.text("rp", "Rp", Tx::r(x2 - sl.measure("amount", &amount, Some(size), None) - k("rp_gap"), cy).cy());
        if i + 1 < funds.len() {
            hair(sl, mx, k("rows_top") + (i + 1) as f32 * pitch, x2 - mx);
        }
    }
    double_rule(sl, mx, k("total_rule_y"), x2 - mx);
    let sum = fit::digits(d.total());
    let (label_w, rp_w) = (sl.measure("total_label", &d.w.total_gifts, None, None), sl.measure("total_rp", "Rp", None, None));
    let z = sl.must(largest(sl.ty_size("total"), k("total_floor"), |z| sl.measure("total", &sum, Some(z), None) + k("total_rp_gap") + rp_w + label_w + 48.0 <= x2 - mx), k("total_floor"))?;
    let width = sl.measure("total", &sum, Some(z), None);
    sl.text("total_label", &d.w.total_gifts, Tx::l(mx, k("total_cy")).cy());
    sl.text("total", &sum, Tx::r(x2, k("total_cy")).cy().size(z));
    sl.text("total_rp", "Rp", Tx::r(x2 - width - k("total_rp_gap"), k("total_cy")).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bot"));
    let k = sl.sec("s04");
    chrome(sl, d, x2, None);
    sl.text("tag", &d.title, Tx::r(x2, sl.g("aside_cy")).cy().maxw(x2 - mx));
    let (wide, number_max) = if have.qris { (k("w_qr"), k("number_max_qr")) } else { (k("w"), k("number_max")) };
    let a = &d.account;
    let nsz = sl.must(largest(number_max, k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide), k("number_floor"))?;
    let hsz = sl.must(largest(sl.ty_size("holder"), 40.0, |z| sl.measure("holder", &a.holder, Some(z), None) <= wide), 40.0)?;
    let y = k("stack_top");
    sl.text("bank", &a.bank, Tx::l(mx, y).cy().maxw(wide));
    let base = y + 52.0 + sl.cap(&sl.face_of("number"), nsz);
    sl.text("number", &a.number, Tx::l(mx, base).size(nsz).maxw(wide));
    sl.push(R::new(mx, base + 36.0, wide, sl.g("rule_thin")).fill("ink"));
    sl.text("caption", &d.w.holder, Tx::l(mx, base + 100.0).cy());
    let holder = base + 176.0;
    sl.text("holder", &a.holder, Tx::l(mx, holder).cy().size(hsz).maxw(wide));
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), sl.ty_size("note"), wide) };
    let mut ny = holder + 90.0;
    for line in &notes {
        sl.text("note", line, Tx::l(mx, ny).cy().maxw(wide));
        ny += k("line");
    }
    sl.need(notes.is_empty() || ny - k("line") <= bottom)?;
    if have.qris {
        // The code is dark on light, so it keeps a light plate in every colourway, inside a double-rule frame.
        let (frame, pad, top) = (k("frame"), k("frame_pad"), k("qr_top"));
        let fx = x2 - frame;
        sl.push(R::new(fx, top, frame, frame).border(4.0, "ink"));
        sl.push(R::new(fx + 12.0, top + 12.0, frame - 24.0, frame - 24.0).border(sl.g("rule_thin"), "ink"));
        sl.push(R::new(fx + pad, top + pad, frame - 2.0 * pad, frame - 2.0 * pad).fill("card"));
        let inner = k("plate_pad");
        sl.image(Pic::Qris, fx + pad + inner, top + pad + inner, frame - 2.0 * (pad + inner), frame - 2.0 * (pad + inner), 0.0, true, Pos::Mid);
        vrule(sl, fx - k("gap"), top, top + frame);
        sl.text("caption", &d.w.scan, Tx::c(fx + frame / 2.0, top + frame + 44.0).cy().maxw(frame));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, bottom) = (sl.g("mx"), sl.g("body_bot"));
    let k = sl.sec("s05");
    let (px, text_w) = (super::W - k("pic_w"), if have.photo { k("text_w_pic") } else { k("text_w") });
    let right = if have.photo { px - k("pic_gap") } else { sl.g("x2") };
    chrome(sl, d, right, d.date_long.as_deref());
    if have.photo {
        sl.image(Pic::Photo, px, 0.0, k("pic_w"), super::H, 0.0, false, Pos::Mid);
        vrule(sl, px - k("pic_gap") / 2.0, 130.0, bottom);
    }
    sl.text("caption", &d.w.verse, Tx::l(mx, k("cap_y")).cy());
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(text_w));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: text_w, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, bottom) = (sl.g("mx"), sl.g("body_bot"));
    let k = sl.sec("s06");
    let px = super::W - sl.g("mx") - k("pic_w");
    let right = if have.photo { px - k("pic_gap") } else { sl.g("x2") };
    chrome(sl, d, sl.g("x2"), d.date_long.as_deref());
    if have.photo {
        let ph = bottom - k("pic_top");
        if have.photo2 {
            let half = (ph - 24.0) / 2.0;
            sl.image(Pic::Photo, px, k("pic_top"), k("pic_w"), half, 0.0, false, Pos::Mid);
            sl.image(Pic::Photo2, px, k("pic_top") + half + 24.0, k("pic_w"), half, 0.0, false, Pos::Mid);
        } else {
            sl.image(Pic::Photo, px, k("pic_top"), k("pic_w"), ph, 0.0, false, Pos::Mid);
        }
        vrule(sl, px - k("pic_gap") / 2.0, k("pic_top"), bottom);
    }
    sl.text("caption", &d.w.notice, Tx::l(mx, k("kick_y")).cy());
    let wide = right - mx;
    let (ls, face) = (sl.ty("n_title").ls, sl.face_of("n_title"));
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::balanced(&words, wide * 0.96, space, &word, &word).0.iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("top") + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let y0 = k("top") + cap + lines.len().saturating_sub(1) as f32 * tl + k("gap_after");
    // The body flows down one column beside a picture, else down two.
    let cols = if have.photo { 1 } else { 2 };
    let colw = (wide - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let (body_face, body_size) = (sl.face_of("n_body"), sl.ty_size("n_body"));
    let blh = (body_size * k("body_lh")).round();
    let (mut col, mut y) = (0, y0);
    for para in d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, colw)).collect::<Vec<_>>() {
        for line in &para {
            if y > bottom {
                col += 1;
                y = y0;
            }
            sl.need(col < cols)?;
            sl.text("n_body", line, Tx::l(mx + col.min(cols - 1) as f32 * (colw + k("col_gap")), y).maxw(colw));
            y += blh;
        }
        y += k("line_gap");
    }
    if col > 0 {
        vrule(sl, mx + colw + k("col_gap") / 2.0, y0 - body_size, bottom);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn birthdays(count: usize) -> Slide {
        let text: Vec<String> = (0..count).map(|at| format!("Ibu Contoh Nama Lengkap {at}\t{} November", at * 28 / count.max(1) + 1)).collect();
        Slide { notice: NoticeKind::Birthdays, heading: "Ulang Tahun".into(), text: text.join("\n"), marked: (7..14.min(count)).collect(), ..Slide::default() }
    }

    #[test]
    fn thirty_names_are_one_slide_and_more_continue() {
        assert_eq!(paginate(&birthdays(30), NoticeStyle::Magazine).len(), 1);
        assert!(paginate(&birthdays(60), NoticeStyle::Magazine).len() > 1);
    }

    #[test]
    fn thirty_names_across_two_months_with_a_head_in_each_column_are_one_slide() {
        let date = |at: u8| artosia_core::calendar::Date { year: 2026, month: if at < 9 { 11 } else { 12 }, day: if at < 9 { 22 + at } else { at - 8 } };
        let text: Vec<String> = (0..30u8).map(|at| format!("Ibu Contoh Nama Lengkap {at}\t{} X", date(at).day)).collect();
        let slide = Slide { notice: NoticeKind::Birthdays, heading: "Ulang Tahun".into(), text: text.join("\n"), dates: (0..30).map(date).collect(), ..Slide::default() };
        assert_eq!(paginate(&slide, NoticeStyle::Magazine).len(), 1);
    }

    #[test]
    fn a_week_of_eight_rows_in_four_days_is_one_slide() {
        let text = "Minggu\t07.00\tIbadah Raya I\n\t09.30\tIbadah Raya II dan Sekolah Minggu\nRabu\t18.30\tPersekutuan Doa dan Pemahaman Alkitab\n\t20.00\tLatihan Paduan Suara dan Tim Pemusik\nJumat\t17.30\tPelayanan Kunjungan Kasih ke Rumah Sakit\n\t19.00\tIbadah Doa Puasa Bersama Seluruh Jemaat\nSabtu\t09.00\tRapat Majelis dan Pengurus Komisi Anak\n\t16.00\tBakti Sosial dan Pembagian Sembako Jemaat";
        let slide = Slide { notice: NoticeKind::Schedule, heading: "Jadwal Pekan Ini".into(), text: text.into(), marked: vec![5], ..Slide::default() };
        assert_eq!(paginate(&slide, NoticeStyle::Magazine).len(), 1);
    }
}
