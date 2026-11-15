//! Direction 6, Serif italic: one serif in roman and italic on warm paper, italic date numerals, and one colour wash rising from the bottom edge.

use artosia_core::deck::NoticeKind;

use super::grown_size;
use super::data::{Data, Have, Name};
use super::fit;
use super::{Fits, H, LINE, Pic, Pos, R, Sheet, Split, Tm, Tx, VerseBox, W, fit_cells, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The ground, the wash, the top line (the church news at the left, `right` or else the church at the right) and its rule.
fn page(sl: &mut Sheet, d: &Data, right: &str) {
    let (mx, x2, ky, ry, rw, wy, wh) = (sl.g("mx"), sl.g("x2"), sl.g("kicker_y"), sl.g("rule_y"), sl.g("rule_w"), sl.g("wash_y"), sl.g("wash_h"));
    sl.push(R::new(0.0, 0.0, W, H).fill("paper"));
    sl.push(R::new(0.0, wy, W, wh).fill("grad:wash"));
    let right = if right.is_empty() { d.church.as_str() } else { right };
    let mut taken = 0.0;
    if !right.is_empty() {
        taken = sl.text("kicker", right, Tx::r(x2, ky).maxw((x2 - mx) / 2.0)).w + 40.0;
    }
    let left = sl.text("kicker", &d.w.warta, Tx::l(mx, ky).maxw(x2 - mx - taken - sl.mark_reserve()));
    sl.mark_after(left.x + left.w);
    sl.push(R::new(mx, ry, x2 - mx, rw).fill("ink"));
}

/// The title at the left and `right` (role, words) at the right of its baseline; the title and the left edge of what the right leaves.
fn masthead(sl: &mut Sheet, title: &str, right: Option<(&str, &str)>) -> (Tm, f32) {
    let (mx, x2, base) = (sl.g("mx"), sl.g("x2"), sl.g("title_base"));
    let edge = match right.filter(|(_, words)| !words.is_empty()) {
        Some((role, words)) => x2 - sl.text(role, words, Tx::r(x2, base).maxw((x2 - mx) * 0.55)).w - 40.0,
        None => x2,
    };
    (sl.text("title", title, Tx::l(mx, base).maxw(edge - mx)), edge)
}

/// The highlight: a tint band with an accent bar at its left.
fn band(sl: &mut Sheet, x: f32, y: f32, width: f32, height: f32) {
    let bar = sl.g("bar_w");
    sl.push(R::new(x, y, width, height).fill("tint"));
    sl.push(R::new(x, y, bar, height).fill("accent"));
}

fn rule(sl: &mut Sheet, x: f32, y: f32, width: f32) {
    sl.push(R::new(x, y - 0.75, width, 1.5).fill("rule"));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, band_pad) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    page(sl, d, "");
    masthead(sl, &d.title, Some(("subtitle", d.week_short.as_str())));
    // The rows of one day are one group, headed by its date.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        match groups.last_mut() {
            Some((first, len)) if rows[*first].day == r.day && rows[*first].date == r.date => *len += 1,
            _ => groups.push((i, 1)),
        }
    }
    let (gap, group_min, ratio, avail) = (k("group_gap"), k("group_min"), k("ratio"), bottom - top);
    let height = |z: f32, extra: f32| groups.iter().map(|(_, len)| (*len as f32 * (ratio * z + extra)).max(group_min)).sum::<f32>() + groups.len().saturating_sub(1) as f32 * gap;
    let (fig_x, time_x, event_x) = (k("fig_x"), k("time_x"), k("event_x"));
    let ev_w = x2 - event_x - if rows.iter().any(|r| r.special) { k("tag_room") } else { 24.0 };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, k("size_max"), k("size_floor"), |z, e| height(z, e) <= avail).ok_or(Split)?;
    let pitch = ratio * size + extra;
    let (fig_size, day_size) = (sl.ty_size("fig"), sl.ty_size("day_name"));
    let mut y = top;
    for (at, (first, len)) in groups.iter().enumerate() {
        if at > 0 {
            rule(sl, mx, y - gap / 2.0, x2 - mx);
        }
        for r in rows[*first..first + len].iter().enumerate().filter(|(_, r)| r.special).map(|(j, _)| j) {
            band(sl, mx - band_pad, y + r as f32 * pitch + 3.0, x2 - mx + 2.0 * band_pad, pitch - 6.0);
        }
        let head = &rows[*first];
        let base = y + 0.67 * fig_size + 6.0;
        if head.date > 0 {
            sl.text("fig", &head.date.to_string(), Tx::l(fig_x, base).maxw(time_x - fig_x - 16.0));
        }
        sl.text("day_name", &head.day, Tx::l(fig_x, base + 14.0 + 0.7 * day_size).maxw(time_x - fig_x - 16.0));
        for j in 0..*len {
            let (r, lines) = (&rows[first + j], &cells[first + j]);
            let cy = y + j as f32 * pitch + pitch / 2.0;
            if r.special {
                sl.text("tag", &d.w.special, Tx::r(x2, cy).cy());
            }
            sl.text("time", &r.time, Tx::l(time_x, cy - (lines.len() as f32 - 1.0) / 2.0 * LINE * size).cy().size(size).maxw(event_x - time_x - 16.0));
            put_lines(sl, if r.special { "event_hl" } else { "event" }, lines, event_x, cy, size, ev_w);
        }
        y += (*len as f32 * pitch).max(group_min) + gap;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, base) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("title_base"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, d, "");
    let (title, edge) = masthead(sl, &d.title, d.highlighted().then_some(("ribbon", d.week_label.as_str())));
    let at = title.x + title.w + 40.0;
    sl.text("subtitle", &d.month, Tx::l(at, base).maxw((edge - at).max(200.0)));
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let month_w = if d.spans { names.iter().map(|x| sl.measure("bday_mon", &x.month, None, None)).fold(0.0, f32::max) } else { 0.0 };
    let day_cells = |sl: &Sheet, z: f32| sl.measure("bday_day_hl", "00", Some(z), None) + if d.spans { k("day_gap") + month_w } else { 0.0 };
    let role_of = |name: &Name| if name.hl { "name_hl" } else { "name" };
    let holds = |z: f32| {
        let room = colw - day_cells(sl, z) - k("name_gap");
        names.iter().all(|name| fit::ladder(&name.name, room, &|s| sl.measure(role_of(name), s, Some(z), None)).is_some())
    };
    let size = largest(grown_size(k("size_max"), k("size_max") * 1.7, avail, per, k("ratio")), k("size_floor"), holds).ok_or(Split)?;
    let (num_w, name_x) = (sl.measure("bday_day_hl", "00", Some(size), None), day_cells(sl, size) + k("name_gap"));
    let name_w = colw - name_x;
    let pitch = (avail / per as f32).min(k("ratio") * 1.35 * size);
    // The highlight goes first so the names sit on it; a run that crosses the column break is drawn in both columns.
    for c in 0..cols {
        let marked: Vec<usize> = (c * per..((c + 1) * per).min(n)).filter(|i| names[*i].hl).map(|i| i - c * per).collect();
        if let (Some(first), Some(last)) = (marked.first(), marked.last()) {
            band(sl, mx + c as f32 * (colw + k("col_gap")) - k("band_over"), top + *first as f32 * pitch + 3.0, colw + 2.0 * k("band_over"), (last - first + 1) as f32 * pitch - 6.0);
        }
    }
    let cap = sl.cap(&sl.face_of("name"), size);
    for (i, name) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let by = top + r as f32 * pitch + pitch / 2.0 + cap / 2.0;
        let role = role_of(name);
        let (text, _) = name_in(sl, &name.name, role, name_w, size);
        sl.text(if name.hl { "bday_day_hl" } else { "bday_day" }, &format!("{:02}", name.day), Tx::r(cx + num_w, by).size(size));
        if d.spans {
            sl.text("bday_mon", &name.month, Tx::l(cx + num_w + k("day_gap"), by));
        }
        sl.text(role, &text, Tx::l(cx + name_x, by).size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, &d.w.offerings);
    masthead(sl, &d.title, d.date_long.as_deref().map(|date| ("subtitle", date)));
    let total_y = bottom - k("total_h");
    let avail = total_y - k("total_gap") - top;
    let rp_w = sl.measure("rp", "Rp", None, None);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    // The amount column is measured first; a label gives way to it, an amount never does.
    let digits_at = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let label_room = |z: f32| x2 - mx - k("no_w") - rp_w - k("rp_gap") - digits_at(sl, z) - 48.0;
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, label_room, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let (amount_w, label_w) = (digits_at(sl, size), label_room(size));
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let cap = sl.cap(&sl.face_of("fund"), size);
    for (i, f) in funds.iter().enumerate() {
        let by = top + i as f32 * pitch + (pitch + cap) / 2.0;
        sl.text("fund_no", &format!("{:02}", i + 1), Tx::l(mx, by));
        put_lines(sl, "fund", &cells[i], mx + k("no_w"), by - cap / 2.0, size, label_w);
        sl.text("amount", &fit::digits(f.rupiah), Tx::r(x2, by).size(size));
        sl.text("rp", "Rp", Tx::r(x2 - amount_w - k("rp_gap"), by));
        if i + 1 < funds.len() {
            rule(sl, mx, top + (i + 1) as f32 * pitch, x2 - mx);
        }
    }
    // The total is the sum of the funds, set apart under a rule; a very long sum makes it smaller, never cut.
    sl.push(R::new(mx, total_y, x2 - mx, 2.0).fill("ink"));
    let tb = total_y + 40.0 + sl.cap(&sl.face_of("total"), sl.ty_size("total")) + 14.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(mx, tb - 8.0).maxw((x2 - mx) * 0.4));
    let total_rp = sl.measure("total_rp", "Rp", None, None);
    let total = sl.text("total", &fit::digits(d.total()), Tx::r(x2, tb).maxw(x2 - mx - label.w - total_rp - 24.0 - 48.0));
    sl.text("total_rp", "Rp", Tx::r(total.x - 24.0, tb));
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2) = (sl.g("mx"), sl.g("x2"));
    let k = sl.sec("s04");
    page(sl, d, &d.title);
    let (card, pad) = (k("card"), k("card_pad"));
    let wide = if have.qris { x2 - mx - card - k("col_gap") } else { x2 - mx };
    let a = &d.account;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let y = k("stack_top");
    sl.text("bank", &a.bank, Tx::l(mx, y).maxw(wide));
    let nb = y + 44.0 + sl.cap(&sl.face_of("number"), nsz) + 24.0;
    sl.text("number", &a.number, Tx::l(mx, nb).size(nsz).maxw(wide));
    sl.push(R::new(mx, nb + 40.0, wide, 2.0).fill("ink"));
    sl.text("atas", &d.w.holder, Tx::l(mx, nb + 104.0));
    sl.text("holder", &a.holder, Tx::l(mx, nb + 180.0).size(hsz).maxw(wide));
    let note_size = sl.ty_size("note");
    let lh = (note_size * 1.4).round();
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let mut ny = nb + 244.0;
    for line in &notes {
        sl.text("note", line, Tx::l(mx, ny).maxw(wide));
        ny += lh;
    }
    // The notes stay clear of the bottom edge.
    sl.need(ny - lh <= H - 40.0)?;
    if have.qris {
        // The code is dark on light, so it keeps a light plate in every colourway.
        let (qx, qy) = (x2 - card, 310.0);
        sl.push(R::new(qx, qy, card, card).fill("card"));
        sl.image(Pic::Qris, qx + pad, qy + pad, card - 2.0 * pad, card - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(qx + card / 2.0, qy + card + 56.0).maxw(card + 96.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, d, d.date_long.as_deref().unwrap_or(""));
    sl.text("caption", &d.w.verse, Tx::l(mx, k("ref_y") - 84.0).fill("accent").maxw(k("text_w")));
    sl.text("ref", &d.reference, Tx::l(mx, k("ref_y")).maxw(k("text_w")));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx, top: k("top"), w: k("text_w"), room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, d, d.date_long.as_deref().unwrap_or(""));
    let wide = if have.photo { x2 - mx - k("photo_w") - k("photo_gap") } else { k("text_w") };
    if have.photo {
        sl.image(Pic::Photo, x2 - k("photo_w"), k("photo_y"), k("photo_w"), k("photo_h"), 0.0, false, Pos::Mid);
    }
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title);
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::balanced(&words, wide, space, &word, &word).0.iter().map(|line| fit::join(line)).collect()
    };
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.4).round();
    let body: Vec<Vec<String>> = d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect();
    let body_h = body.iter().map(Vec::len).sum::<usize>() as f32 * blh + body.len() as f32 * k("line_gap");
    // The title is the largest on at most two lines whose stack, with the body under it, holds.
    let (kicker_y, top) = (332.0, 380.0);
    let stack = |z: f32, lines: usize| top + sl.cap(&face, z) + lines.saturating_sub(1) as f32 * k("title_lh") * z + 56.0 + 4.0 + 64.0 + body_h;
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| {
        let lines = wrap_at(z).len();
        lines <= 2 && stack(z, lines) <= bottom + 40.0
    }), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    sl.text("caption", &d.w.notice, Tx::l(mx, kicker_y).fill("accent"));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, top + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let mut y = top + cap + (lines.len() as f32 - 1.0) * tl + 56.0;
    sl.push(R::new(mx, y, 96.0, 4.0).fill("accent"));
    y += 4.0 + 64.0;
    for para in &body {
        for line in para {
            sl.text("n_body", line, Tx::l(mx, y).maxw(wide));
            y += blh;
        }
        y += k("line_gap");
    }
    // A body taller than the page does not fit at this size.
    sl.need(y - k("line_gap") <= bottom + 40.0)?;
    Ok(())
}
