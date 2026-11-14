"""Candidate B, Swiss Numerals: the six kinds. A flush-left grid, the date numeral in the one dark column, rows on hairlines."""
from engine import Sheet, Split, digits, largest, W, H
from kit import draw_verse, fit_events, groups, hl_runs, lines_for, name_grid, qr, title_lines, verse_lines


def chrome(sh, d, right=None):
    g = sh.g
    sh.rect(0, 0, W, H, fill="paper", id="ground")
    sh.rect(g("mx"), g("strip_y"), g("x2") - g("mx"), g("strip_w"), fill="ink", id="strip")
    sh.text("kicker", d["words"]["warta"], g("mx"), g("kicker_y"), id="kicker")
    sh.text("kicker", right or d["church"], g("x2"), g("kicker_y"), anchor="r", id="kicker.r")


def title(sh, text, id="title"):
    """Two tones in two weights: the first word light and muted, the rest bold and dark."""
    g = sh.g
    words = text.split()
    if len(words) == 1:
        return sh.text("title", text, g("mx"), g("title_base"), id=id)
    a = sh.text("title_a", words[0], g("mx"), g("title_base"), id=id + ".a")
    sp = sh.measure("title", " ")
    b = sh.text("title", " ".join(words[1:]), a["x"] + a["w"] + sp, g("title_base"), id=id + ".b")
    return {"x": a["x"], "w": b["x"] + b["w"] - a["x"]}


def foot(sh, part):
    if part:
        sh.text("foot", part, sh.g("x2") / 2 + sh.g("mx") / 2, sh.g("kicker_y"), anchor="c", id="foot")


def s01(sh, d, part=None):
    g, k, s = sh.g, sh.k("s01"), d["s01"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d)
    title(sh, s["title"])
    sh.text("subtitle", s["week_short"], x2, g("title_base"), anchor="r", id="subtitle")
    rows, gs = s["rows"], groups(s["rows"])
    top, bottom = g("body_top"), g("body_bottom")
    ev_w = lambda r: x2 - k("event_x") - (k("tag_room") if r["special"] else 24)

    def height(z, extra=0.0):
        return sum(max(len(r) * (k("ratio") * z + extra), k("group_min")) for _, r in gs) + (len(gs) - 1) * k("group_gap")

    got = fit_events(sh, rows, "event", "event_hl", ev_w, k("size_max"), k("size_floor"), lambda z, e: height(z, e) <= bottom - top)
    sh.need(got is not None, "schedule rows")
    z, lines, extra = got
    pitch = k("ratio") * z + extra
    cap = sh.cap("event", z)
    bx = mx + k("block_w")
    y, ri, texts = top, 0, []
    for gi, (i0, grp) in enumerate(gs):
        gh = max(len(grp) * pitch, k("group_min"))
        sh.rect(mx, y, k("block_w"), gh, fill="block", id=f"g{gi}.block")
        for j, r in enumerate(grp):
            ry = y + j * pitch
            if r["special"]:
                sh.rect(bx, ry, x2 - bx, pitch, fill="accent", id=f"g{gi}.r{j}.band")
            else:
                sh.rect(bx, ry + pitch - 0.75 if j + 1 < len(grp) else y + gh - 0.75, x2 - bx, 1.5, fill="rule", id=f"g{gi}.r{j}.rule")
        texts.append((gi, y, gh, grp, ri))
        ri += len(grp)
        y += gh + k("group_gap")
    for gi, y, gh, grp, ri in texts:
        sh.text("fig", str(grp[0]["date"]), mx + 24, y + 22 + 0.7 * sh.size_of("fig"), id=f"g{gi}.fig")
        sh.text("day_name", grp[0]["day"], mx + 24, y + 22 + 0.7 * sh.size_of("fig") + 18 + 0.72 * sh.size_of("day_name"), id=f"g{gi}.day")
        for j, r in enumerate(grp):
            ry = y + j * pitch
            ev = lines[ri + j]
            mid = ry + pitch / 2
            hl = r["special"]
            sh.text("time_hl" if hl else "time", r["time"], k("time_x"), mid - (len(ev) - 1) / 2 * 1.12 * z + cap / 2, size=z, id=f"g{gi}.r{j}.time")
            for li, ln in enumerate(ev):
                sh.text("event_hl" if hl else "event", ln, k("event_x"), mid + (li - (len(ev) - 1) / 2) * 1.12 * z + cap / 2, size=z, id=f"g{gi}.r{j}.event{li}")
            if hl:
                sh.text("tag", d["words"]["special"], x2 - 24, mid + sh.cap("tag") / 2, anchor="r", id=f"g{gi}.r{j}.tag")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "height": round(height(z, extra), 1), "room": bottom - top, "two_line_rows": sum(len(l) > 1 for l in lines)})
    foot(sh, part)


def s02(sh, d, key="s02", part=None, caption=None):
    g, k, s = sh.g, sh.k("s02"), d[key]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d)
    t = title(sh, s["title"])
    names = s["names"]
    sub = sh.text("subtitle", s["month"], t["x"] + t["w"] + 40, g("title_base"), id="month")
    rb = sh.text("ribbon", s["week_label"], x2, g("title_base"), anchor="r", id="range")
    sh.need(rb["x"] > sub["x"] + sub["w"] + 24, "title row")
    cols = next(c for lim, c in k("cols") if len(names) <= lim)
    spans = s["spans"]
    mon_w = max((sh.measure("bday_mon", n["month"]) for n in names), default=0) if spans else 0
    day_cells = lambda z: sh.measure("bday_day_hl", "00", z) + ((k("day_gap") + mon_w) if spans else 0)
    cells, z, pitch, colw = name_grid(sh, names, cols, g("body_top"), g("body_bottom"), mx, x2, k("col_gap"), "name", "name_hl", k("size_max"), k("size_floor"), k("ratio"), day_cells, k("name_gap"))
    per = -(-len(names) // cols)
    num_w = sh.measure("bday_day_hl", "00", z)
    for cl in cells:
        sh.rect(cl["cx"], cl["cy"] + pitch / 2 - 0.75, colw, 1.5, fill="rule", id=f"n{cl['i']}.rule")
    for c, r0, r1 in hl_runs(names, cells, per):
        cx = mx + c * (colw + k("col_gap"))
        sh.rect(cx - k("band_over"), g("body_top") + r0 * pitch, colw + 2 * k("band_over"), (r1 - r0 + 1) * pitch, fill="accent", id=f"band.c{c}")
    cap = sh.cap("name", z)
    for cl in cells:
        nm = names[cl["i"]]
        base = cl["cy"] + cap / 2
        hl = nm["hl"]
        sh.text("bday_day_hl" if hl else "bday_day", f"{nm['day']:02d}", cl["cx"] + num_w, base, anchor="r", size=z, id=f"n{cl['i']}.day")
        if spans:
            sh.text("bday_mon_hl" if hl else "bday_mon", nm["month"], cl["cx"] + num_w + k("day_gap"), base, id=f"n{cl['i']}.mon")
        sh.text("name_hl" if hl else "name", cl["text"], cl["cx"] + day_cells(z) + k("name_gap"), base, size=z, id=f"n{cl['i']}.name")
    foot(sh, part and (part + ("   " + caption if caption else "")))


def s03(sh, d):
    g, k, s = sh.g, sh.k("s03"), d["s03"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, d["words"]["offerings"])
    title(sh, s["title"])
    sh.text("subtitle", s["date_long"], x2, g("title_base"), anchor="r", id="subtitle")
    funds = s["funds"]
    n = len(funds)
    top = g("body_top")
    total_y = g("body_bottom") - k("total_h")
    avail = total_y - k("total_gap") - top
    rp_w = sh.measure("rp", "Rp")
    amount_w = lambda z: max(sh.measure("amount", digits(f["rupiah"]), z) for f in funds)
    label_room = lambda z: x2 - mx - rp_w - k("rp_gap") - amount_w(z) - 48
    z = largest(k("size_max"), k("size_floor"), lambda z: n * k("ratio") * z <= avail and all(sh.measure("fund", f["label"], z) <= label_room(z) for f in funds))
    sh.need(z is not None, "fund rows")
    pitch = min(avail / n, 1.9 * z)
    cap = sh.cap("fund", z)
    for i, f in enumerate(funds):
        sh.rect(mx, top + (i + 1) * pitch - 0.75, x2 - mx, 1.5, fill="rule", id=f"f{i}.rule")
    for i, f in enumerate(funds):
        base = top + i * pitch + (pitch + cap) / 2
        sh.text("fund", f["label"], mx, base, size=z, id=f"f{i}.label")
        sh.text("amount", digits(f["rupiah"]), x2, base, anchor="r", size=z, id=f"f{i}.amount")
        sh.text("rp", "Rp", x2 - amount_w(z) - k("rp_gap"), base, anchor="r", id=f"f{i}.rp")
    sh.rect(mx, total_y, x2 - mx, 2, fill="ink", id="total.rule")
    total = sum(f["rupiah"] for f in funds)
    rpw = sh.measure("total_rp", "Rp")
    tz = largest(sh.size_of("total"), 96, lambda z: sh.measure("total", digits(total), z) + rpw + 24 <= x2 - mx)
    sh.need(tz is not None, "total")
    tw = sh.measure("total", digits(total), tz)
    base = total_y + 56 + 30 + 0.72 * tz
    sh.text("total_label", d["words"]["total_gifts"], mx, total_y + 56, id="total.label")
    sh.text("total", digits(total), x2, base, anchor="r", size=tz, id="total")
    sh.text("total_rp", "Rp", x2 - tw - 24, base, anchor="r", id="total.rp")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "amount_w": round(amount_w(z), 1), "label_room": round(label_room(z), 1), "total_w": round(tw + rpw + 24, 1), "total_size": tz})


def s04(sh, d, qr_on=True):
    g, k, s = sh.g, sh.k("s04"), d["s04"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["title"])
    card = k("card") if qr_on else 0
    wide = (x2 - mx - card - k("col_gap")) if qr_on else x2 - mx
    nsz = largest(k("number_max"), k("number_floor"), lambda z: sh.measure("number", s["number"], z) <= wide)
    hsz = largest(sh.size_of("holder"), k("holder_floor"), lambda z: sh.measure("holder", s["holder"], z) <= wide)
    sh.need(nsz is not None and hsz is not None, "account")
    y = k("stack_top")
    sh.text("bank", s["bank"], mx, y, id="bank")
    nb = y + 36 + 0.72 * nsz
    sh.text("number", s["number"], mx, nb, size=nsz, id="number")
    sh.rect(mx, nb + 36, wide, 2, fill="ink", id="number.rule")
    sh.text("atas", d["words"]["holder"], mx, nb + 36 + 60, id="atas")
    sh.text("holder", s["holder"], mx, nb + 36 + 60 + 72, size=hsz, id="holder")
    ny = nb + 36 + 60 + 72 + 64
    for i, line in enumerate(s["note"]):
        for j, ln in enumerate(lines_for(sh, "note", line, wide, sh.size_of("note"))):
            sh.text("note", ln, mx, ny, id=f"note{i}.{j}")
            ny += 54
    if qr_on:
        qx, qy = x2 - card, 290
        qr(sh, qx, qy, card, "ink", "card", "paper", k("card_pad"))
        sh.text("caption", d["words"]["scan"], qx + card / 2, qy + card + 56, anchor="c", id="qr.caption")
    sh.fit.update({"number_size": nsz, "number_w": round(sh.measure("number", s["number"], nsz), 1), "wide": round(wide, 1), "bottom": round(ny, 1)})


def s05(sh, d):
    g, k, s = sh.g, sh.k("s05"), d["s05"]
    mx = g("mx")
    chrome(sh, d, s["date_long"])
    sh.text("caption", d["words"]["verse"], mx, k("ref_y") - 120, id="kicker2")
    sh.text("ref", s["reference"], mx, k("ref_y"), id="ref")
    room = k("bottom") - k("top")
    z = largest(k("size_max"), k("size_floor"), lambda z: len(verse_lines(sh, s["verses"], k("text_w"), z, k("sup_scale"))) * k("lh") * z <= room)
    sh.need(z is not None, "verse")
    lines = verse_lines(sh, s["verses"], k("text_w"), z, k("sup_scale"))
    draw_verse(sh, lines, mx, k("top"), z, k("lh"), "verse", "verse_n", "accent", k("sup_scale"), k("sup_rise"), id="verse")
    sh.fit.update({"size": z, "lines": len(lines), "height": round(len(lines) * k("lh") * z, 1), "room": room})


def s06(sh, d, photo=True):
    g, k, s = sh.g, sh.k("s06"), d["s06"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["date_long"])
    width = (x2 - mx - k("photo_w") - k("photo_gap")) if photo else 1500
    if photo:
        sh.rect(x2 - k("photo_w"), 160, k("photo_w"), k("photo_h"), fill="rule", id="photo")
        sh.text("caption", "foto", x2 - k("photo_w") / 2, 160 + k("photo_h") / 2, anchor="c", id="photo.label")
    bsz = sh.size_of("n_body")
    body = [lines_for(sh, "n_body", p, width, bsz) for p in s["paragraphs"]]
    nb = sum(len(b) for b in body)
    top = 300

    def stack(z):
        _, lines = title_lines(sh, "n_title", s["title"], width * 0.94, 2, z, z)
        return top + 0.72 * z + (len(lines) - 1) * k("title_lh") * z + 96 + nb * 66 + len(body) * k("line_gap")

    z0, _ = title_lines(sh, "n_title", s["title"], width * 0.94, 2, k("title_max"), k("title_floor"))
    z = largest(z0, k("title_floor"), lambda z: stack(z) <= g("body_bottom"))
    sh.need(z is not None, "notice")
    z, lines = title_lines(sh, "n_title", s["title"], width * 0.94, 2, z, z)
    sh.text("caption", d["words"]["notice"], mx, 232, id="kicker2")
    y = top
    for i, ln in enumerate(lines):
        sh.text("n_title_b" if i == len(lines) - 1 else "n_title", ln, mx, y + 0.72 * z + i * k("title_lh") * z, size=z, id=f"title.{i}")
    y += 0.72 * z + (len(lines) - 1) * k("title_lh") * z + 96
    for pi, bl in enumerate(body):
        for j, ln in enumerate(bl):
            sh.text("n_body", ln, mx, y, id=f"p{pi}.{j}")
            y += 66
        y += k("line_gap")
    sh.fit.update({"title_size": z, "title_lines": len(lines), "bottom": round(y, 1)})


LAYOUT = {"s01": s01, "s02": s02, "s03": s03, "s04": s04, "s05": s05, "s06": s06}
