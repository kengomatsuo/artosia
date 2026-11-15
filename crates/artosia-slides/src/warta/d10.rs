//! Direction 10, Diagonal: a heavy caps headline cut by one diagonal into a deep colour block, white date tiles and a slanted total plate.

use artosia_core::deck::NoticeKind;

use super::data::{Data, Have};
use super::fit;
use super::{Fits, H, Item, Pic, Pos, R, Sheet, Split, Tx, VerseBox, W, fit_cells, grown_size, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse};

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

/// The part of a long card at the foot; on the money slide it sits on the total plate.
pub fn foot(sl: &mut Sheet, note: &str, on_plate: bool) {
    let (x, y) = (sl.g("mx"), sl.g("foot_y"));
    sl.text("foot", note, Tx::l(x, y).cy().fill(if on_plate { "on_brand" } else { "mute" }));
}

/// A strip between two parallel slanted edges, as a rect turned to them: it runs from `p0` to `p1`
/// and `thick` toward `side` (1 left of the way down, -1 right), `ext` longer at both ends.
fn slab(sl: &mut Sheet, p0: (f32, f32), p1: (f32, f32), side: f32, thick: f32, ext: f32, fill: &str) {
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let len = dx.hypot(dy);
    let (ux, uy) = (dx / len, dy / len);
    let (cx, cy) = (p0.0 + ux * len / 2.0 - uy * side * thick / 2.0, p0.1 + uy * len / 2.0 + ux * side * thick / 2.0);
    let (w, h) = (len + 2.0 * ext, thick);
    sl.push(R::new(cx - w / 2.0, cy - h / 2.0, w, h).fill(fill).rot(uy.atan2(ux).to_degrees()));
}

/// A parallelogram with level top and bottom, `wd` wide, its left edge from (`xt`, `y0`) to (`xb`, `y1`).
/// The turned strip overshoots its ends; what lands inside the sheet is covered with the ground.
fn para(sl: &mut Sheet, xt: f32, xb: f32, y0: f32, y1: f32, wd: f32, fill: &str) {
    let ext = 30.0;
    let (dx, dy) = (xb - xt, y1 - y0);
    let len = dx.hypot(dy);
    let thick = wd * dy / len;
    slab(sl, (xt, y0), (xb, y1), -1.0, thick, ext, fill);
    let band = ext * dy / len + thick * dx.abs() / len + 4.0;
    let (left, right) = (xt.min(xb) - ext, xt.max(xb) + wd + ext);
    if y0 > 0.0 {
        sl.push(R::new(left, y0 - band, right - left, band).fill("ground"));
    }
    if y1 < H {
        sl.push(R::new(left, y1, right - left, band).fill("ground"));
    }
}

/// Where the header block's diagonal edge is at height `y`.
fn edge(sl: &Sheet, y: f32) -> f32 {
    sl.g("hdr_w") - sl.g("cut") * y / sl.g("hdr_h")
}

/// The width a text of `role` centred on `cy` may take before it meets the diagonal.
fn room(sl: &Sheet, role: &str, cy: f32) -> f32 {
    let cap = sl.cap(&sl.face_of(role), sl.ty_size(role));
    edge(sl, cy + cap / 2.0) - sl.g("mx") - sl.g("edge_pad")
}

/// The ground, the header block with its diagonal and parallel stripe, and the kicker line.
fn page(sl: &mut Sheet, kicker: &str) {
    let (mx, hw, hh, cut, gap, sw) = (sl.g("mx"), sl.g("hdr_w"), sl.g("hdr_h"), sl.g("cut"), sl.g("gap"), sl.g("stripe_w"));
    sl.push(R::new(0.0, 0.0, W, H).fill("ground"));
    sl.push(R::new(0.0, 0.0, hw - cut, hh).fill("brand_dk"));
    slab(sl, (hw, 0.0), (hw - cut, hh), 1.0, 112.0, 0.0, "brand_dk");
    para(sl, hw + gap, hw + gap - cut, 0.0, hh, sw, "brand");
    let (ky, maxw) = (sl.g("kicker_cy"), room(sl, "kicker", sl.g("kicker_cy")) - sl.mark_reserve());
    let left = sl.text("kicker", kicker, Tx::l(mx, ky).cy().maxw(maxw));
    sl.mark_after(left.x + left.w);
}

/// The title and the line under it, both on the header block.
fn header(sl: &mut Sheet, title: &str, sub: Option<&str>) {
    let (mx, ty, sy) = (sl.g("mx"), sl.g("title_cy"), sl.g("sub_cy"));
    let maxw = room(sl, "title", ty);
    sl.text("title", title, Tx::l(mx, ty).cy().maxw(maxw));
    if let Some(sub) = sub.filter(|sub| !sub.is_empty()) {
        let maxw = room(sl, "subtitle", sy);
        sl.text("subtitle", sub, Tx::l(mx, sy).cy().maxw(maxw));
    }
}

/// The date range at the right of the header row, led by an arrow.
fn chip(sl: &mut Sheet, label: &str) {
    let (x2, cy, hh, hw, cut, gap, sw) = (sl.g("x2"), sl.g("chip_cy"), sl.g("hdr_h"), sl.g("hdr_w"), sl.g("cut"), sl.g("gap"), sl.g("stripe_w"));
    let (aw, half, head, agap) = (sl.g("arrow_w"), sl.g("arrow_half"), sl.g("arrow_head"), sl.g("arrow_gap"));
    let stripe_edge = hw + gap + sw - cut * cy / hh;
    let t = sl.text("ribbon", label, Tx::r(x2, cy).cy().maxw(x2 - (stripe_edge + 30.0) - aw - agap));
    let x0 = t.x - agap - aw;
    let hx = x0 + aw - head;
    // The head is a square turned on its corner with its left half covered by the ground.
    let side = head * std::f32::consts::SQRT_2;
    sl.push(R::new(hx - side / 2.0, cy - side / 2.0, side, side).fill("brand").rot(45.0));
    sl.push(R::new(hx - head - 2.0, cy - head - 1.0, head + 2.0, head - half + 1.0).fill("ground"));
    sl.push(R::new(hx - head - 2.0, cy + half, head + 2.0, head - half + 1.0).fill("ground"));
    sl.push(R::new(x0, cy - half, aw - head + 2.0, 2.0 * half).fill("brand"));
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, rx) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("rx"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    page(sl, &d.church_kicker(""));
    header(sl, &d.title, Some(&d.week_short));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || rows[i].day != rows[i - 1].day).collect();
    let gaps = firsts.iter().filter(|first| **first).count().saturating_sub(1) as f32 * k("group_gap");
    let avail = bottom - top;
    let tile_w = rows.iter().zip(&firsts).filter(|(_, first)| **first).map(|(r, _)| sl.measure("day_name", &r.day, None, None)).fold(0.0, f32::max) + 2.0 * k("day_pad");
    let time_x = mx + tile_w + k("day_gap");
    let time_w = rows.iter().map(|r| sl.measure("time", &r.time, None, None)).fold(0.0, f32::max);
    let event_x = time_x + time_w + k("time_gap");
    let tag_w = if rows.iter().any(|r| r.special) { sl.measure("tag", &d.w.special, None, None) + 2.0 * k("tag_pad") } else { 0.0 };
    let ev_w = x2 - event_x - if tag_w > 0.0 { tag_w + k("tag_gap") } else { 0.0 };
    let hi = if n as f32 <= k("sparse_rows") { k("sparse_size_max") } else { k("size_max") };
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event_hl", &events, |_| ev_w, hi, k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) + gaps <= avail).ok_or(Split)?;
    let pitch = (k("ratio_max") * size + extra).min((avail - gaps) / n as f32);
    let (day_cap, date_cap) = (sl.cap(&sl.face_of("day_name"), sl.ty_size("day_name")), sl.cap(&sl.face_of("date_small"), sl.ty_size("date_small")));
    let mut y = top;
    for (i, r) in rows.iter().enumerate() {
        if firsts[i] {
            if i > 0 {
                y += k("group_gap");
            }
            // One dark tile spans the rows of its day.
            let count = rows[i..].iter().take_while(|next| next.day == r.day).count();
            let (ty, th) = (y + 3.0, count as f32 * pitch - 6.0);
            sl.push(R::new(mx, ty, tile_w, th).fill("brand_dk").rx(rx));
            let date = (r.date > 0).then(|| format!("{} {}", r.date, r.month.chars().take(3).collect::<String>()));
            let cx = mx + tile_w / 2.0;
            match date.filter(|_| th >= day_cap + date_cap + 56.0) {
                Some(date) => {
                    let first = ty + (th - (day_cap + 14.0 + date_cap)) / 2.0 + day_cap;
                    sl.text("day_name", &r.day, Tx::c(cx, first).maxw(tile_w - 12.0));
                    sl.text("date_small", &date, Tx::c(cx, first + 14.0 + date_cap).maxw(tile_w - 12.0));
                }
                None => {
                    sl.text("day_name", &r.day, Tx::c(cx, ty + th / 2.0).cy().maxw(tile_w - 12.0));
                }
            }
        }
        let cy = y + pitch / 2.0;
        if r.special {
            sl.push(R::new(time_x - 14.0, y + 3.0, x2 + sl.g("band_pad") - time_x + 14.0, pitch - 6.0).fill("tint").rx(rx));
        }
        let time = Tx::l(time_x, cy).cy().maxw(time_w + 4.0);
        sl.text("time", &r.time, if r.special { time.fill("brand") } else { time });
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if r.special {
            sl.push(R::new(x2 - tag_w, cy - k("tag_h") / 2.0, tag_w, k("tag_h")).fill("brand").rx(rx));
            sl.text("tag", &d.w.special, Tx::l(x2 - tag_w + k("tag_pad"), cy).cy());
        }
        let next_in_day = i + 1 < rows.len() && !firsts[i + 1];
        if next_in_day && !r.special && !rows[i + 1].special {
            sl.push(R::new(time_x - 14.0, y + pitch - sl.g("rule_w") / 2.0, x2 - time_x + 14.0, sl.g("rule_w")).fill("line"));
        }
        y += pitch;
    }
    Ok(())
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom, band_pad, rx) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"), sl.g("band_pad"), sl.g("rx"));
    let k = sl.sec("s02");
    let names = &d.names;
    let n = names.len().max(1);
    page(sl, &d.church_kicker(""));
    header(sl, &d.title, Some(&d.month));
    if d.highlighted() {
        chip(sl, &d.week_label);
    }
    let cols = sl.list("s02", "cols").into_iter().find(|row| n as f32 <= row[0]).map(|row| row[1] as usize).ok_or(Split)?;
    let per = n.div_ceil(cols);
    let colw = ((x2 - mx) - k("col_gap") * (cols as f32 - 1.0)) / cols as f32;
    let avail = bottom - top;
    let floor_day = sl.ty_size("bday_day");
    let day_at = |z: f32| (z * k("day_ratio")).round().clamp(floor_day, k("day_max").max(floor_day));
    // The tile is as wide as the widest day with its month.
    let tile_at = |z: f32| {
        let widest = if d.spans { names.iter().map(|x| sl.measure("bday_day_hl", &d.day_of(x), Some(day_at(z)), None)).fold(0.0, f32::max) + 2.0 * k("tile_pad") } else { 0.0 };
        widest.max(k("tile_w"))
    };
    let hi = grown_size(k("size_max"), k("sparse_size_max"), avail, per, k("ratio"));
    let fits = |z: f32| {
        let name_w = colw - tile_at(z) - k("tile_gap");
        names.iter().all(|nm| fit::ladder(&nm.name, name_w, &|s| sl.measure("name_hl", s, Some(z), None)).is_some())
    };
    let size = sl.must(largest(hi, k("size_floor"), fits), k("size_floor"))?;
    let (tile_w, ds) = (tile_at(size), day_at(size));
    let name_w = colw - tile_w - k("tile_gap");
    let pitch = (avail / per as f32).min(1.9 * size);
    let tile_h = (size * k("tile_ratio")).round().max(k("tile_h")).min(pitch - 8.0);
    // One tint band per run of highlighted names in a column, drawn first so the tiles sit on it.
    for c in 0..cols {
        let rows_in = n.saturating_sub(c * per).min(per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let mut r = 0;
        while r < rows_in {
            if !names[c * per + r].hl {
                r += 1;
                continue;
            }
            let start = r;
            while r < rows_in && names[c * per + r].hl {
                r += 1;
            }
            sl.push(R::new(cx - band_pad, top + start as f32 * pitch + 3.0, colw + 2.0 * band_pad, (r - start) as f32 * pitch - 6.0).fill("tint").rx(rx));
        }
    }
    let (name_cap, day_cap) = (sl.cap(&sl.face_of("name"), size), sl.cap(&sl.face_of("bday_day"), ds));
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / per, i % per);
        let cx = mx + c as f32 * (colw + k("col_gap"));
        let cy = top + r as f32 * pitch + pitch / 2.0;
        let tile = R::new(cx, cy - tile_h / 2.0, tile_w, tile_h).rx(rx);
        sl.push(if nm.hl { tile.fill("brand") } else { tile.fill("panel").border(sl.g("rule_w"), "line") });
        sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &d.day_of(nm), Tx::c(cx + tile_w / 2.0, cy + day_cap / 2.0).size(ds));
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, role, name_w, size);
        sl.text(role, &text, Tx::l(cx + tile_w + k("tile_gap"), cy + name_cap / 2.0).size(size).maxw(name_w));
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, rx) = (sl.g("mx"), sl.g("x2"), sl.g("rx"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, &d.church_kicker(""));
    header(sl, &d.title, d.date_long.as_deref());
    // The plate is reserved first: a slanted edge on the header's slope, brand to the foot of the sheet.
    let (tt, pad) = (sl.g("total_top"), sl.g("total_pad"));
    let th = H - tt;
    let run = th * sl.g("cut") / sl.g("hdr_h");
    sl.push(R::new(run, tt, W - run, th).fill("brand"));
    slab(sl, (run, tt), (0.0, H), -1.0, 90.0, 0.0, "brand");
    let (top, avail) = (k("rows_top"), k("rows_bot") - k("rows_top"));
    let rp_w = sl.measure("rp", "Rp", None, None);
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let digits_at = |sl: &Sheet, z: f32| funds.iter().map(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None)).fold(0.0, f32::max);
    let label_x = mx + k("no_w") + k("no_gap");
    let lab_at = |z: f32| x2 - (rp_w + k("rp_gap") + digits_at(sl, z)) - k("lab_gap") - label_x;
    let (size, cells, extra) = fit_cells(sl, "fund", &labels, lab_at, k("size_max"), k("size_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let label_w = lab_at(size);
    let pitch = (k("ratio_max") * size + extra).min(avail / n as f32);
    let tile = k("no_w").min(pitch - 8.0);
    for (i, f) in funds.iter().enumerate() {
        let cy = top + i as f32 * pitch + pitch / 2.0;
        sl.push(R::new(mx, cy - tile / 2.0, tile, tile).fill("brand").rx(rx));
        sl.text("fund_no", &(i + 1).to_string(), Tx::c(mx + tile / 2.0, cy).cy());
        put_lines(sl, "fund", &cells[i], label_x, cy, size, label_w);
        let amount = fit::digits(f.rupiah);
        let aw = sl.measure("amount", &amount, Some(size), None);
        sl.text("amount", &amount, Tx::r(x2, cy).cy().size(size));
        sl.text("rp", "Rp", Tx::r(x2 - aw - k("rp_gap"), cy).cy());
        if i + 1 < funds.len() {
            let w = sl.g("rule_w");
            sl.push(R::new(mx, top + (i + 1) as f32 * pitch - w / 2.0, x2 - mx, w).fill("line"));
        }
    }
    let cy = tt + th / 2.0;
    let label = sl.text("total_label", &d.w.total_gifts, Tx::l(run + pad, cy).cy());
    let (rp, rp_gap) = (sl.measure("total_rp", "Rp", None, None), sl.g("total_rp_gap"));
    let total = sl.text("total", &fit::digits(d.total()), Tx::r(x2, cy).cy().maxw(x2 - (run + pad + label.w + 48.0) - rp - rp_gap));
    sl.text("total_rp", "Rp", Tx::r(total.x - rp_gap, cy).cy());
    Ok(())
}

/// A number cut in two lines at the space nearest its middle, else at its middle.
fn halves(number: &str) -> Vec<String> {
    let parts: Vec<&str> = number.split_whitespace().collect();
    if parts.len() >= 2 {
        let at = (1..parts.len()).min_by_key(|at| (parts[..*at].join(" ").len() as isize - parts[*at..].join(" ").len() as isize).abs()).unwrap_or(1);
        return vec![parts[..at].join(" "), parts[at..].join(" ")];
    }
    let chars: Vec<char> = number.chars().collect();
    let mid = chars.len().div_ceil(2);
    vec![chars[..mid].iter().collect(), chars[mid..].iter().collect()]
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), sl.g("x2"), sl.g("body_top"), sl.g("body_bottom"));
    let k = sl.sec("s04");
    page(sl, &d.church_kicker(""));
    header(sl, &d.title, Some(&d.w.account));
    let wide = if have.qris { x2 - mx - k("card") - k("frame_off") - k("col_gap") } else { x2 - mx };
    let a = &d.account;
    let one = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= wide);
    // Under the floor the number breaks into two groups and never gets smaller.
    let (lines, nsz) = match one {
        Some(z) => (vec![a.number.clone()], z),
        None => {
            let lines = halves(&a.number);
            let z = largest(k("number_max"), k("number_floor"), |z| lines.iter().all(|line| sl.measure("number", line, Some(z), None) <= wide)).unwrap_or(k("number_floor"));
            (lines, z)
        }
    };
    let hsz = largest(sl.ty_size("holder"), k("holder_floor"), |z| sl.measure("holder", &a.holder, Some(z), None) <= wide).unwrap_or(k("holder_floor"));
    let (bcap, ncap) = (sl.cap(&sl.face_of("bank"), sl.ty_size("bank")), sl.cap(&sl.face_of("number"), nsz));
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, wide) };
    let nlh = (note_size * 1.4).round();
    let nstep = (ncap * 1.3).round();
    let stack = bcap + 40.0 + ncap + (lines.len() - 1) as f32 * nstep + 44.0 + 62.0 + 70.0 + 14.0 + if notes.is_empty() { 0.0 } else { 40.0 + notes.len() as f32 * nlh };
    let at = top + ((bottom - top) - stack) / 2.0;
    sl.need(at >= top - 10.0)?;
    sl.text("bank", &a.bank, Tx::l(mx, at + bcap).maxw(wide));
    let mut nb = at + bcap + 40.0 + ncap;
    for line in &lines {
        sl.text("number", line, Tx::l(mx, nb).size(nsz).maxw(wide));
        nb += nstep;
    }
    nb -= nstep;
    let w = sl.g("rule_w");
    sl.push(R::new(mx, nb + 44.0 - w / 2.0, wide, w).fill("line"));
    sl.text("atas", &d.w.holder, Tx::l(mx, nb + 44.0 + 62.0));
    let hb = nb + 44.0 + 62.0 + 70.0;
    sl.text("holder", &a.holder, Tx::l(mx, hb).size(hsz).maxw(wide));
    let mut yy = hb + 14.0 + 40.0 + nlh * 0.8;
    for line in &notes {
        sl.text("note", line, Tx::l(mx, yy).maxw(wide));
        yy += nlh;
    }
    if have.qris {
        // The code is black on white with its quiet zone and is never recoloured.
        let (card, off, pad) = (k("card"), k("frame_off"), k("card_pad"));
        let (cx, cy) = (x2 - card - off, top + ((bottom - top) - (card + off + 60.0)) / 2.0);
        sl.push(R::new(cx + off, cy + off, card, card).border(k("frame_w"), "brand"));
        sl.push(R::new(cx, cy, card, card).fill("#FFFFFF"));
        sl.image(Pic::Qris, cx + pad, cy + pad, card - 2.0 * pad, card - 2.0 * pad, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(cx + (card + off) / 2.0, cy + card + off + 52.0).maxw(card + off + 120.0));
    }
    Ok(())
}

/// A picture on the right, its left edge cut on the header's slope; the ground covers what the cut leaves.
fn photo(sl: &mut Sheet, x: f32, top: f32) {
    sl.image(Pic::Photo, x, top, W - x, H - top, 0.0, false, Pos::Mid);
    let (high, run) = (H - top, (H - top) * sl.g("cut") / sl.g("hdr_h"));
    slab(sl, (x + run, top), (x, H), 1.0, run * high / run.hypot(high) + 4.0, 0.0, "ground");
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let mx = sl.g("mx");
    let k = sl.sec("s05");
    page(sl, &d.church_kicker(""));
    if have.photo {
        photo(sl, k("pic_x"), k("pic_top"));
    }
    header(sl, &d.title, None);
    if !d.reference.is_empty() {
        let sy = sl.g("sub_cy");
        let maxw = room(sl, "ref", sy);
        sl.text("ref", &d.reference, Tx::l(mx, sy).cy().maxw(maxw));
    }
    let x = mx + k("bar_w") + k("bar_gap");
    let w = if have.photo { k("text_w_pic") } else { k("text_w") };
    let face = sl.face_of("verse");
    let from = sl.items.len();
    let z = verse(sl, &d.verses, &VerseBox { x, top: k("top"), w, room: k("bottom") - k("top"), font: &face, section: "s05" })?;
    // The bar stands as tall as the lines that were set.
    let bases: Vec<f32> = sl.items[from..].iter().filter_map(|item| if let Item::Text(t) = item { (t.role == "verse").then_some(t.y) } else { None }).collect();
    let step = k("lh") * z;
    let lines = match (bases.iter().copied().reduce(f32::min), bases.iter().copied().reduce(f32::max)) {
        (Some(first), Some(last)) => ((last - first) / step).round() + 1.0,
        _ => 1.0,
    };
    sl.push(R::new(mx, k("top"), k("bar_w"), lines * step).fill("brand"));
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, bottom) = (sl.g("mx"), sl.g("body_bottom"));
    let k = sl.sec("s06");
    page(sl, &d.kicker(&d.w.notice));
    if have.photo {
        photo(sl, k("pic_x"), k("pic_top"));
    } else {
        // Without a picture a second stripe fills the corner.
        let (xt, y0) = (k("stripe_x"), k("stripe_top"));
        para(sl, xt, xt - (H - y0) * sl.g("cut") / sl.g("hdr_h"), y0, H, sl.g("stripe_w"), "brand");
    }
    header(sl, &d.w.notice, d.date_long.as_deref());
    let wide = if have.photo { k("text_w_pic") } else { k("text_w") };
    let ls = sl.ty("n_title").ls;
    let face = sl.face_of("n_title");
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3), k("title_floor"))?;
    let lines = wrap_at(z);
    let (tl, cap) = (z * k("title_lh"), sl.cap(&face, z));
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, k("title_top") + cap + i as f32 * tl).size(z).maxw(wide));
    }
    let bar_y = k("title_top") + cap + (lines.len() as f32 - 1.0) * tl + k("body_gap");
    sl.push(R::new(mx, bar_y, k("bar_w"), k("bar_h")).fill("brand"));
    let mut y = bar_y + k("bar_h") + k("body_gap");
    let body_face = sl.face_of("n_body");
    let body_size = sl.ty_size("n_body");
    let blh = (body_size * 1.4).round();
    let mut last = y;
    for para in d.paragraphs.iter().map(|p| paragraph(sl, p, &body_face, body_size, wide)).collect::<Vec<_>>() {
        for line in &para {
            sl.text("n_body", line, Tx::l(mx, y + blh * 0.8).maxw(wide));
            last = y + blh;
            y += blh;
        }
        y += k("line_gap");
    }
    // A body taller than the page does not fit at this size.
    sl.need(last <= bottom)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warta::{Item, build};
    use artosia_core::calendar::Date;
    use artosia_core::deck::{NoticeStyle, Slide};

    /// Thirty invented names over 22 Nov to 20 Des, the library spec's fixture shape.
    fn thirty() -> Slide {
        let day = |at: usize| if at < 9 { Date { year: 2026, month: 11, day: 22 + at as u8 } } else { Date { year: 2026, month: 12, day: (at - 8) as u8 } };
        let names: Vec<String> = (0..30).map(|at| format!("Bpk. Daniel Hutapea Sihombing {at}\t{} Nov", day(at).day)).collect();
        Slide { notice: NoticeKind::Birthdays, heading: "Ulang Tahun".into(), text: names.join("\n"), dates: (0..30).map(day).collect(), ..Slide::default() }
    }

    #[test]
    fn thirty_names_over_two_months_hold_with_the_title_at_its_size() {
        let sheet = build(&thirty(), NoticeStyle::Diagonal, Have { photo: false, photo2: false, qris: false }, None, false).expect("holds");
        let title = sheet.items.iter().find_map(|item| if let Item::Text(t) = item { (t.role == "title").then_some(t.size) } else { None }).expect("a title");
        assert!((title - 92.0).abs() <= 2.0, "title {title}");
    }

    #[test]
    fn a_long_account_number_breaks_into_two_groups_and_a_short_one_stays_whole() {
        assert_eq!(halves("1234 5678 9012 3456"), ["1234 5678", "9012 3456"]);
        assert_eq!(halves("1234567890123456"), ["12345678", "90123456"]);
    }
}
