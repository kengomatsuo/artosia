//! Direction 16, Figures: a ruled register on paper, condensed numerals for dates, a colour bar per week,
//! one hue per fund from chip to bar, and a full-width signal foot where the total is the largest thing.

use artosia_core::deck::{NoticeKind, Slide};

use super::data::{Data, Have};
use super::fit;
use super::{Fits, H, Pic, Pos, R, Sheet, Split, Tx, W, fit_cells, largest, largest_step, name_in, paragraph, paragraphs, put_lines, verse, VerseBox};

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

/// The part of a long card at the foot, in ink on the signal foot of an offering and muted elsewhere.
pub fn foot(sl: &mut Sheet, slide: &Slide, note: &str) {
    let (x, cy) = (sl.g("mx"), 1054.0);
    if slide.notice == NoticeKind::Money {
        sl.text("foot", note, Tx::l(x, cy).cy().fill("on_signal"));
    } else {
        sl.text("foot", note, Tx::l(x, cy).cy());
    }
}

fn fund(at: usize) -> String {
    format!("fund{}", at % 6 + 1)
}

fn rule(sl: &mut Sheet, x: f32, y: f32, w: f32) {
    sl.push(R::new(x, y, w, sl.g("rule_w")).fill("ink").op(sl.g("rule_op")));
}

/// The kicker at the top left, and the season's sign at the right end of its line.
fn kicker(sl: &mut Sheet, words: &str) {
    let (mx, x2) = (sl.g("mx"), W - sl.g("mx"));
    sl.text("kicker", words, Tx::l(mx, sl.g("kicker_cy")).cy().maxw(x2 - mx - sl.mark_reserve()));
    sl.mark_here(x2 - sl.mk("size") / 2.0, sl.mk("cy"));
}

/// The ground, the kicker, the title with what follows it, and the chip at the top right.
fn page(sl: &mut Sheet, d: &Data, title: &str, sub: Option<&str>, tag: Option<&str>) {
    let (mx, x2) = (sl.g("mx"), W - sl.g("mx"));
    sl.push(R::new(0.0, 0.0, W, H).fill("ground"));
    kicker(sl, &d.church_kicker(""));
    let (gap, base) = (sl.g("sub_gap"), sl.g("title_base"));
    let tag_w = tag.map_or(0.0, |tag| sl.measure("tag", tag, None, None) + 2.0 * sl.g("tag_pad"));
    let room = x2 - mx - if tag.is_some() { tag_w + 30.0 } else { 0.0 };
    let sub_min = sub.map_or(0.0, |sub| sl.measure("sub", sub, Some(32.0), None) + gap);
    let t = sl.text("title", title, Tx::l(mx, base).maxw((room - sub_min).max(500.0)));
    if let Some(sub) = sub {
        sl.text("sub", sub, Tx::l(t.x + t.w + gap, base).maxw((room - t.w - gap).max(200.0)));
    }
    if let Some(tag) = tag {
        let (cy, h) = (sl.g("tag_cy"), sl.g("tag_h"));
        sl.push(R::new(x2 - tag_w, cy - h / 2.0, tag_w, h).fill("signal"));
        sl.text("tag", tag, Tx::l(x2 - tag_w + sl.g("tag_pad"), cy).cy());
    }
}

/// A column head over a cell block: the head words, a heavy rule and the cell under them.
fn block(sl: &mut Sheet, x: f32, w: f32, h: f32) {
    let (hy, hh, rh) = (sl.g("head_y"), sl.g("head_h"), sl.g("rule_head"));
    sl.push(R::new(x, hy + hh - rh, w, rh).fill("ink"));
    sl.push(R::new(x, hy + hh, w, h).fill("cell"));
}

fn head(sl: &mut Sheet, word: &str, x: f32, anchor: At) {
    let cy = sl.g("head_y") + sl.g("head_h") / 2.0;
    let o = match anchor {
        At::Start => Tx::l(x, cy),
        At::Middle => Tx::c(x, cy),
        At::End => Tx::r(x, cy),
    };
    sl.text("kicker", word, o.cy());
}

enum At {
    Start,
    Middle,
    End,
}

fn s01(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), W - sl.g("mx"), sl.g("body_top"), sl.g("body_bot"));
    let k = sl.sec("s01");
    let rows = &d.rows;
    let n = rows.len().max(1);
    let week = (!d.week_short.is_empty()).then_some(d.week_short.as_str());
    page(sl, d, &d.title, week, Some(&d.w.wib));
    let firsts: Vec<bool> = (0..rows.len()).map(|i| i == 0 || (rows[i].day.as_str(), rows[i].date) != (rows[i - 1].day.as_str(), rows[i - 1].date)).collect();
    let numeral = |r: &super::data::Row| if r.date > 0 { r.date.to_string() } else { r.dow.clone() };
    let (pad, gap) = (sl.g("day_pad") + 10.0, sl.g("mon_gap"));
    let date_w = rows.iter().map(|r| sl.measure("day", &numeral(r), None, None) + gap + sl.measure("date", &r.dow, None, None)).fold(0.0, f32::max);
    let date_cell = (pad + date_w + sl.g("day_pad")).max(sl.g("day_w"));
    let time_x = mx + date_cell + 12.0;
    let time_w = rows.iter().map(|r| sl.measure("time", &r.time, None, None)).fold(0.0, f32::max).min(360.0);
    let event_x = time_x + time_w + 40.0;
    let tag_room = if rows.iter().any(|r| r.special) { sl.measure("tag", &d.w.special, None, None) + 2.0 * sl.g("tag_pad") + 24.0 } else { 24.0 };
    let ev_w = k("event_w").min(x2 - event_x - tag_room);
    let avail = bottom - top;
    let events: Vec<&str> = rows.iter().map(|r| r.event.as_str()).collect();
    let (size, cells, extra) = fit_cells(sl, "event", &events, |_| ev_w, sl.ty_size("event"), sl.g("name_floor"), |z, e| n as f32 * (k("ratio") * z + e) <= avail).ok_or(Split)?;
    let pitch = (2.0 * size + extra).min(avail / n as f32);
    block(sl, mx, x2 - mx, pitch * n as f32);
    head(sl, &d.w.col_date, mx + pad, At::Start);
    let hl_sw = sl.g("hl_sw");
    for (i, r) in rows.iter().enumerate() {
        let cy = top + (i as f32 + 0.5) * pitch;
        if r.special {
            sl.push(R::new(mx + 10.0, cy - pitch / 2.0, x2 - mx - 10.0, pitch).fill("signal").border(hl_sw, "ink"));
        }
    }
    for (i, r) in rows.iter().enumerate() {
        let (y, cy) = (top + i as f32 * pitch, top + (i as f32 + 0.5) * pitch);
        if firsts[i] {
            let t = sl.text(if r.special { "day_on" } else { "day" }, &numeral(r), Tx::l(mx + pad, cy).cy().maxw(date_cell - pad));
            if r.date > 0 {
                let role = if r.special { "date_on" } else { "date" };
                sl.text(role, &r.dow, Tx::l(t.x + t.w + gap, cy).cy());
            }
        }
        sl.text(if r.special { "time_hl" } else { "time" }, &r.time, Tx::l(time_x, cy).cy().maxw(time_w));
        put_lines(sl, if r.special { "event_hl" } else { "event" }, &cells[i], event_x, cy, size, ev_w);
        if r.special {
            sl.text("tag", &d.w.special, Tx::r(x2 - sl.g("tag_pad"), cy).cy().fill("on_signal"));
        }
        if rows.get(i + 1).is_some_and(|next| !r.special && !next.special) {
            let from = if firsts[i + 1] { mx + 10.0 } else { mx + date_cell };
            rule(sl, from, y + pitch, x2 - from);
        }
    }
    sl.need(top + pitch * n as f32 <= bottom + 0.5)
}

fn s02(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), W - sl.g("mx"), sl.g("body_top"), sl.g("body_bot"));
    let names = &d.names;
    let n = names.len();
    let (per, bar, gutter) = (sl.g("per") as usize, sl.g("week_bar"), sl.g("gutter"));
    let cols = n.div_ceil(per).max(1);
    if cols > 3 {
        return Err(Split);
    }
    let rows = n.div_ceil(cols).max(1);
    let colw = (x2 - mx - gutter * (cols as f32 - 1.0)) / cols as f32;
    let tag = d.highlighted().then_some(d.week_label.as_str());
    page(sl, d, &d.title, Some(&d.month), tag);
    let avail = bottom - top;
    let month_of = |name: &super::data::Name| if name.month.is_empty() { d.month.split(' ').next().filter(|_| !d.month.contains('\u{2013}')).map(|m| m.chars().take(3).collect()).unwrap_or_default() } else { name.month.clone() };
    let (hi, lo) = (super::grown_size(sl.ty_size("name"), 56.0, avail, rows, 1.9), sl.g("name_floor"));
    let (day_pad, gap, name_pad) = (sl.g("day_pad"), sl.g("mon_gap"), sl.g("name_pad"));
    // The date cell grows with the numeral, which grows with the names, so both are searched together.
    let numeral_at = |z: f32| (z * 1.4).max(sl.ty_size("bday_day"));
    let month_at = |z: f32| (z * 0.8).max(sl.ty_size("mon"));
    let cell_at = |z: f32| {
        let w = names.iter().map(|x| sl.measure("bday_day", &format!("{:02}", x.day), Some(numeral_at(z)), None) + gap + sl.measure("mon", &month_of(x), Some(month_at(z)), None)).fold(0.0, f32::max);
        (bar + day_pad + w + day_pad).max(bar + sl.g("day_w"))
    };
    let holds = |z: f32| {
        let width = colw - cell_at(z) - name_pad;
        names.iter().all(|x| fit::ladder(&x.name, width, &|s| sl.measure("name_hl", s, Some(z), None)).is_some())
    };
    let size = sl.must(largest(hi, lo, holds), lo)?;
    let (cell, pitch) = (cell_at(size), (avail / rows as f32).min(1.9 * size));
    let name_w = colw - cell - name_pad;
    let (nz, mz) = (numeral_at(size), month_at(size));
    for c in 0..cols {
        let (x, count) = (mx + c as f32 * (colw + gutter), rows.min(n - c * rows));
        block(sl, x, colw, pitch * count as f32);
        head(sl, &d.w.col_date, x + bar + day_pad, At::Start);
        head(sl, &d.w.col_name, x + cell + name_pad, At::Start);
        let ids = c * rows..c * rows + count;
        // The week bar, one rectangle per run of a week.
        let mut at = ids.start;
        while at < ids.end {
            let end = (at..ids.end).find(|i| d.weeks[*i] != d.weeks[at]).unwrap_or(ids.end);
            sl.push(R::new(x, top + (at - ids.start) as f32 * pitch, bar, (end - at) as f32 * pitch).fill(&format!("fund{}", d.weeks[at] % 5 + 1)));
            at = end;
        }
        // One filled and outlined rectangle for each run of the highlight, never one per row.
        let mut at = ids.start;
        while at < ids.end {
            if !names[at].hl {
                at += 1;
                continue;
            }
            let end = (at..ids.end).find(|i| !names[*i].hl).unwrap_or(ids.end);
            sl.push(R::new(x + bar, top + (at - ids.start) as f32 * pitch, colw - bar, (end - at) as f32 * pitch).fill("signal").border(sl.g("hl_sw"), "ink"));
            at = end;
        }
    }
    for (i, nm) in names.iter().enumerate() {
        let (c, r) = (i / rows, i % rows);
        let (x, cy) = (mx + c as f32 * (colw + gutter), top + (r as f32 + 0.5) * pitch);
        let same = r > 0 && (names[i - 1].day, &names[i - 1].month) == (nm.day, &nm.month);
        if !same {
            let t = sl.text(if nm.hl { "bday_day_hl" } else { "bday_day" }, &format!("{:02}", nm.day), Tx::l(x + bar + day_pad, cy).cy().size(nz));
            sl.text(if nm.hl { "mon_hl" } else { "mon" }, &month_of(nm), Tx::l(t.x + t.w + gap, cy).cy().size(mz));
        }
        let role = if nm.hl { "name_hl" } else { "name" };
        let (text, _) = name_in(sl, &nm.name, "name_hl", name_w, size);
        sl.text(role, &text, Tx::l(x + cell + name_pad, cy).cy().size(size).maxw(name_w));
        if let Some(next) = names.get(i + 1).filter(|next| r + 1 < rows && !nm.hl && !next.hl) {
            let split = next.day == nm.day && next.month == nm.month;
            let from = if split { x + cell } else { x + bar };
            rule(sl, from, top + (r as f32 + 1.0) * pitch, x + colw - from);
        }
    }
    Ok(())
}

fn s03(sl: &mut Sheet, d: &Data) -> Fits {
    let (mx, x2, top) = (sl.g("mx"), W - sl.g("mx"), sl.g("body_top"));
    let k = sl.sec("s03");
    let funds = &d.funds;
    let n = funds.len().max(1);
    page(sl, d, &d.title, d.date_long.as_deref(), None);
    let (total, avail) = (d.total(), sl.g("rows_bot") - top);
    let (lab_x, bar_x, bar_w, bar_h, row_pad) = (sl.g("lab_x"), sl.g("bar_x"), sl.g("bar_w"), sl.g("bar_h"), sl.g("row_pad"));
    let labels: Vec<&str> = funds.iter().map(|f| f.label.as_str()).collect();
    let lab_w = bar_x - sl.g("lab_gap") - lab_x;
    let (size, cells, _) = fit_cells(sl, "fund", &labels, |_| lab_w, sl.ty_size("fund"), sl.g("label_floor"), |z, e| n as f32 * (1.25 * z + e) <= avail).ok_or(Split)?;
    let pitch = (avail / n as f32).min(k("row_max"));
    let share = |rupiah: i64| if total > 0 { rupiah as f64 / total as f64 } else { 0.0 };
    let pct_of = |rupiah: i64| format!("{:.1}%", share(rupiah) * 100.0).replace('.', ",");
    let pct_x = bar_x + bar_w + sl.g("pct_gap");
    let pct_w = funds.iter().map(|f| sl.measure("pct", &pct_of(f.rupiah), None, None)).fold(0.0, f32::max);
    let rp_w = sl.measure("rp", "Rp", None, None);
    let right = x2 - row_pad;
    let left = pct_x + pct_w + 24.0 + rp_w + 14.0;
    let asz = largest(sl.ty_size("amount"), sl.g("name_floor"), |z| funds.iter().all(|f| sl.measure("amount", &fit::digits(f.rupiah), Some(z), None) <= right - left)).unwrap_or(sl.g("name_floor"));
    block(sl, mx, x2 - mx, pitch * n as f32);
    head(sl, &d.w.col_no, mx + sl.g("chip_pad") + sl.g("chip") / 2.0, At::Middle);
    head(sl, &d.w.col_fund, lab_x, At::Start);
    head(sl, &d.w.col_share, bar_x, At::Start);
    head(sl, &d.w.col_sum, right, At::End);
    let chip = sl.g("chip");
    for (i, f) in funds.iter().enumerate() {
        let cy = top + (i as f32 + 0.5) * pitch;
        let hue = fund(i);
        sl.push(R::new(mx + sl.g("chip_pad"), cy - chip / 2.0, chip, chip).fill(&hue));
        sl.text("chip_num", &(i + 1).to_string(), Tx::c(mx + sl.g("chip_pad") + chip / 2.0, cy).cy());
        put_lines(sl, "fund", &cells[i], lab_x, cy, size, lab_w);
        sl.push(R::new(bar_x, cy - bar_h / 2.0, bar_w, bar_h).fill("ink").op(sl.g("track_op")));
        sl.push(R::new(bar_x, cy - bar_h / 2.0, bar_w * share(f.rupiah) as f32, bar_h).fill(&hue));
        sl.text("pct", &pct_of(f.rupiah), Tx::l(pct_x, cy).cy());
        let amount = fit::digits(f.rupiah);
        let t = sl.text("amount", &amount, Tx::r(right, cy).cy().size(asz));
        sl.text("rp", "Rp", Tx::r(t.x - 14.0, cy).cy());
        if i + 1 < funds.len() {
            rule(sl, mx, top + (i as f32 + 1.0) * pitch, x2 - mx);
        }
    }
    // The foot: the total is solved to the room the label and the Rp leave it.
    let (ty, gap) = (sl.g("total_top"), sl.g("tot_gap"));
    sl.push(R::new(0.0, ty, W, H - ty).fill("signal"));
    let label = d.w.total_gifts.as_str();
    let lw = sl.measure("total_label", label, None, None);
    sl.text("total_label", label, Tx::l(mx, ty + (H - ty) / 2.0).cy().maxw(x2 - mx));
    let figure = fit::digits(total);
    let room = x2 - (mx + lw + gap + sl.g("total_rp_w") + gap);
    let z = largest(sl.g("total_max"), sl.g("total_min"), |z| sl.measure("total", &figure, Some(z), None) <= room).unwrap_or(sl.g("total_min"));
    let cy = ty + (H - ty) / 2.0 + 8.0;
    let t = sl.text("total", &figure, Tx::r(x2, cy).cy().size(z).maxw(room.max(300.0)));
    sl.text("total_rp", "Rp", Tx::r(t.x - gap, cy).cy());
    Ok(())
}

fn s04(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top) = (sl.g("mx"), W - sl.g("mx"), sl.g("body_top"));
    let k = sl.sec("s04");
    page(sl, d, &d.title, None, None);
    let a = &d.account;
    let inner = if have.qris { k("w_qr") } else { k("w") };
    let card_w = inner + 40.0;
    let x0 = mx + 20.0;
    let nsz = largest(k("number_max"), k("number_floor"), |z| sl.measure("number", &a.number, Some(z), None) <= inner).unwrap_or(k("number_floor"));
    let hsz = largest(sl.ty_size("holder"), 40.0, |z| sl.measure("holder", &a.holder, Some(z), None) <= inner).unwrap_or(40.0);
    let ncap = sl.cap(&sl.face_of("number"), nsz);
    let note_size = sl.ty_size("note");
    let notes = if a.note.is_empty() { Vec::new() } else { paragraphs(sl, &a.note, &sl.face_of("note"), note_size, inner) };
    let nlh = (note_size * 1.4).round();
    let heights = [120.0, 62.0 + ncap + 26.0, 126.0, if notes.is_empty() { 0.0 } else { 36.0 + notes.len() as f32 * nlh }];
    let total_h: f32 = heights.iter().sum();
    let qs = (W - mx - (mx + card_w + 20.0) - 48.0).min(640.0);
    let bh = if have.qris { total_h.max(qs + 100.0) } else { total_h };
    sl.need(bh <= sl.g("body_bot") - top)?;
    block(sl, mx, card_w, bh);
    let mut y = top;
    let words = [d.w.account.as_str(), d.w.col_no.as_str(), d.w.holder.as_str()];
    for (i, label) in words.iter().enumerate() {
        sl.text("label", label, Tx::l(x0, y + 32.0).cy().maxw(inner));
        match i {
            0 => sl.text("bank", &a.bank, Tx::l(x0, y + 82.0).cy().maxw(inner)),
            1 => sl.text("number", &a.number, Tx::l(x0, y + 62.0 + ncap / 2.0).cy().size(nsz).maxw(inner)),
            _ => sl.text("holder", &a.holder, Tx::l(x0, y + 86.0).cy().size(hsz).maxw(inner)),
        };
        y += heights[i];
        rule(sl, mx, y, card_w);
    }
    for line in &notes {
        sl.text("note", line, Tx::l(x0, y + 18.0 + nlh / 2.0).cy().maxw(inner));
        y += nlh;
    }
    if have.qris {
        let xq = mx + card_w + 20.0;
        let wq = x2 - xq;
        sl.push(R::new(xq, top, wq, bh).fill("cell"));
        sl.image(Pic::Qris, xq + (wq - qs) / 2.0, top + 24.0, qs, qs, 0.0, true, Pos::Mid);
        sl.text("caption", &d.w.scan, Tx::c(xq + wq / 2.0, top + 24.0 + qs + 40.0).cy().maxw(wq - 24.0));
    }
    Ok(())
}

fn s05(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top) = (sl.g("mx"), W - sl.g("mx"), sl.g("body_top"));
    let k = sl.sec("s05");
    sl.push(R::new(0.0, 0.0, W, H).fill("ground"));
    kicker(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")));
    let text_w = if have.photo { k("w_pic") } else { k("w") };
    let cell_w = text_w + 140.0;
    block(sl, mx, cell_w, sl.g("body_bot") - top);
    // The reference sits on a signal tab at the top left of the cell.
    let (tab_h, tw) = (88.0, sl.measure("ref", &d.reference, None, None).min(cell_w - 52.0) + 52.0);
    let ty = sl.g("head_y") + sl.g("head_h") - sl.g("rule_head") - tab_h;
    sl.push(R::new(mx, ty, tw, tab_h).fill("signal"));
    sl.text("ref", &d.reference, Tx::l(mx + 26.0, ty + tab_h / 2.0).cy().maxw(cell_w - 52.0));
    sl.text("sub", &d.w.verse, Tx::l(mx + tw + sl.g("sub_gap"), ty + tab_h / 2.0).cy().maxw((x2 - mx - tw - 60.0).max(200.0)));
    let face = sl.face_of("verse");
    verse(sl, &d.verses, &VerseBox { x: mx + 70.0, top: k("top"), w: text_w, room: k("bot") - k("top"), font: &face, section: "s05" })?;
    if have.photo {
        let px = mx + cell_w + 20.0;
        let (py, ph) = (top, sl.g("body_bot") - top);
        sl.image(Pic::Photo, px, py, x2 - px, ph, 0.0, false, Pos::Mid);
        sl.push(R::new(px, py, x2 - px, ph).border(sl.g("hl_sw"), "ink"));
    }
    Ok(())
}

fn s06(sl: &mut Sheet, d: &Data, have: Have) -> Fits {
    let (mx, x2, top, bottom) = (sl.g("mx"), W - sl.g("mx"), sl.g("head_y"), sl.g("body_bot"));
    let k = sl.sec("s06");
    sl.push(R::new(0.0, 0.0, W, H).fill("ground"));
    kicker(sl, &d.kicker(d.date_long.as_deref().unwrap_or("")));
    let wide = if have.photo { k("w_pic") } else { k("w") };
    let face = sl.face_of("n_title");
    let ls = sl.ty("n_title").ls;
    let words = fit::words(&d.title.to_uppercase());
    let wrap_at = |z: f32| -> Vec<String> {
        let space = sl.adv(&face, " ", z, 0.0);
        let word = |s: &str| sl.adv(&face, s, z, ls * z);
        fit::wrap(&words, wide, space, &word, &word).iter().map(|line| fit::join(line)).collect()
    };
    let z = sl.must(largest_step(k("title_max"), k("title_floor"), 1.0, |z| wrap_at(z).len() <= 3), k("title_floor"))?;
    let lines = wrap_at(z);
    let tl = z * k("title_lh");
    let mut y = top;
    for (i, line) in lines.iter().enumerate() {
        sl.text("n_title", line, Tx::l(mx, y + (i as f32 + 0.5) * tl).cy().size(z).maxw(wide));
    }
    y += lines.len() as f32 * tl + 20.0;
    sl.push(R::new(mx, y, wide, sl.g("rule_head")).fill("ink"));
    y += sl.g("rule_head") + 24.0;
    // A date strip of two ruled cells: the weekday, then the day and month.
    if let Some((weekday, day, month)) = &d.date_card {
        let (a, b) = (weekday.clone(), format!("{day} {month}"));
        let (wa, wb) = (sl.measure("bank", &a, None, None) + 48.0, sl.measure("holder", &b, None, None) + 48.0);
        sl.push(R::new(mx, y, wa, 84.0).fill("cell").border(sl.g("hl_sw"), "ink"));
        sl.push(R::new(mx + wa, y, wb, 84.0).fill("signal").border(sl.g("hl_sw"), "ink"));
        sl.text("bank", &a, Tx::l(mx + 24.0, y + 42.0).cy());
        sl.text("holder", &b, Tx::l(mx + wa + 24.0, y + 42.0).cy().fill("on_signal"));
        y += 84.0 + 28.0;
    }
    let body_face = sl.face_of("n_body");
    let size = sl.ty_size("n_body");
    let blh = (size * 1.4).round();
    for (pi, para) in d.paragraphs.iter().enumerate() {
        for line in paragraph(sl, para, &body_face, size, wide) {
            y += blh;
            sl.text("n_body", &line, Tx::l(mx, y - blh * 0.25).maxw(wide));
        }
        if pi + 1 < d.paragraphs.len() {
            y += 16.0;
        }
    }
    sl.need(y <= bottom)?;
    if have.photo {
        let px = mx + wide + 40.0;
        let (py, ph) = (sl.g("body_top"), bottom - sl.g("body_top"));
        sl.image(Pic::Photo, px, py, x2 - px, ph, 0.0, false, Pos::Mid);
        sl.push(R::new(px, py, x2 - px, ph).border(sl.g("hl_sw"), "ink"));
    }
    Ok(())
}
