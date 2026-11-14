"""Candidate A, Serif Italic: the six kinds. One serif in roman and italic, one colour wash from the bottom edge, no frames."""
from engine import Sheet, Split, digits, largest, W, H
from kit import draw_verse, fit_events, groups, hl_runs, lines_for, name_grid, qr, title_lines, verse_lines


def chrome(sh, d, right=None):
    g = sh.g
    sh.rect(0, 0, W, H, fill="paper", id="ground")
    sh.rect(0, g("wash_y"), W, g("wash_h"), grad={"name": "wash", "kind": "linear", "stops": [[0, "paper", 0], [1, "wash", 1]]}, id="wash")
    sh.text("kicker", d["words"]["warta"], g("mx"), g("kicker_y"), id="kicker")
    sh.text("kicker", right or d["church"], g("x2"), g("kicker_y"), anchor="r", id="kicker.r")
    sh.rect(g("mx"), g("rule_y"), g("x2") - g("mx"), g("rule_w"), fill="ink", id="rule.top")


def foot(sh, part):
    if part:
        sh.text("foot", part, sh.g("mx"), 1040, id="foot")


def s01(sh, d, part=None):
    g, k, s = sh.g, sh.k("s01"), d["s01"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d)
    sh.text("title", s["title"], mx, g("title_base"), id="title")
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
    y = top
    cap = sh.cap("event", z)
    ri = 0
    for gi, (i0, grp) in enumerate(gs):
        gh = max(len(grp) * pitch, k("group_min"))
        if gi:
            sh.rect(mx, y - k("group_gap") / 2 - 0.75, x2 - mx, 1.5, fill="rule", id=f"rule.g{gi}")
        for j, r in enumerate(grp):
            ry = y + j * pitch
            if r["special"]:
                sh.rect(mx - g("band_pad"), ry + 3, x2 - mx + 2 * g("band_pad"), pitch - 6, fill="tint", id=f"g{gi}.r{j}.band")
                sh.rect(mx - g("band_pad"), ry + 3, g("bar_w"), pitch - 6, fill="accent", id=f"g{gi}.r{j}.bar")
        sh.text("fig", str(grp[0]["date"]), k("fig_x"), y + 0.67 * sh.size_of("fig") + 6, id=f"g{gi}.fig")
        sh.text("day_name", grp[0]["day"], k("fig_x"), y + 0.67 * sh.size_of("fig") + 6 + 14 + 0.7 * sh.size_of("day_name"), id=f"g{gi}.day")
        for j, r in enumerate(grp):
            ry = y + j * pitch
            ev = lines[ri]
            ri += 1
            mid = ry + pitch / 2
            if r["special"]:
                sh.text("tag", d["words"]["special"], x2, mid + sh.cap("tag") / 2, anchor="r", id=f"g{gi}.r{j}.tag")
            sh.text("time", r["time"], k("time_x"), mid - (len(ev) - 1) / 2 * 1.12 * z + cap / 2 - (len(ev) - 1) / 2 * 0, size=z, id=f"g{gi}.r{j}.time")
            for li, ln in enumerate(ev):
                sh.text("event_hl" if r["special"] else "event", ln, k("event_x"), mid + (li - (len(ev) - 1) / 2) * 1.12 * z + cap / 2, size=z, id=f"g{gi}.r{j}.event{li}")
        y += gh + k("group_gap")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "height": round(height(z, extra), 1), "room": bottom - top, "two_line_rows": sum(len(l) > 1 for l in lines)})
    foot(sh, part)


def s02(sh, d, key="s02", part=None, caption=None):
    g, k, s = sh.g, sh.k("s02"), d[key]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d)
    t = sh.text("title", s["title"], mx, g("title_base"), id="title")
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
    for c, r0, r1 in hl_runs(names, cells, per):
        cx = mx + c * (colw + k("col_gap"))
        top = g("body_top") + r0 * pitch
        sh.rect(cx - k("band_over"), top + 3, colw + 2 * k("band_over"), (r1 - r0 + 1) * pitch - 6, fill="tint", id=f"band.c{c}")
        sh.rect(cx - k("band_over"), top + 3, g("bar_w"), (r1 - r0 + 1) * pitch - 6, fill="accent", id=f"bar.c{c}")
    cap = sh.cap("name", z)
    for cl in cells:
        nm = names[cl["i"]]
        base = cl["cy"] + cap / 2
        hl = nm["hl"]
        sh.text("bday_day_hl" if hl else "bday_day", f"{nm['day']:02d}", cl["cx"] + num_w, base, anchor="r", size=z, id=f"n{cl['i']}.day")
        if spans:
            sh.text("bday_mon", nm["month"], cl["cx"] + num_w + k("day_gap"), base, id=f"n{cl['i']}.mon")
        sh.text("name_hl" if hl else "name", cl["text"], cl["cx"] + day_cells(z) + k("name_gap"), base, size=z, id=f"n{cl['i']}.name")
    foot(sh, part and (part + ("   " + caption if caption else "")))


def s03(sh, d):
    g, k, s = sh.g, sh.k("s03"), d["s03"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, d["words"]["offerings"])
    t = sh.text("title", s["title"], mx, g("title_base"), id="title")
    sh.text("subtitle", s["date_long"], x2, g("title_base"), anchor="r", id="subtitle")
    funds = s["funds"]
    n = len(funds)
    top = g("body_top")
    total_y = g("body_bottom") - k("total_h")
    avail = total_y - k("total_gap") - top
    rp_w = sh.measure("rp", "Rp")

    def amount_w(z):
        return max(sh.measure("amount", digits(f["rupiah"]), z) for f in funds)

    def label_room(z):
        return x2 - mx - k("no_w") - rp_w - k("rp_gap") - amount_w(z) - 48

    z = largest(k("size_max"), k("size_floor"), lambda z: n * k("ratio") * z <= avail and all(sh.measure("fund", f["label"], z) <= label_room(z) for f in funds))
    sh.need(z is not None, "fund rows")
    pitch = min(avail / n, 1.9 * z)
    cap = sh.cap("fund", z)
    for i, f in enumerate(funds):
        base = top + i * pitch + (pitch + cap) / 2
        sh.text("fund_no", f"{i + 1:02d}", mx, base, id=f"f{i}.no")
        sh.text("fund", f["label"], mx + k("no_w"), base, size=z, id=f"f{i}.label")
        sh.text("amount", digits(f["rupiah"]), x2, base, anchor="r", size=z, id=f"f{i}.amount")
        sh.text("rp", "Rp", x2 - amount_w(z) - k("rp_gap"), base, anchor="r", id=f"f{i}.rp")
        if i + 1 < n:
            sh.rect(mx, top + (i + 1) * pitch - 0.75, x2 - mx, 1.5, fill="rule", id=f"f{i}.rule")
    sh.rect(mx, total_y, x2 - mx, 2, fill="ink", id="total.rule")
    tz = sh.size_of("total")
    total = sum(f["rupiah"] for f in funds)
    tw = sh.measure("total", digits(total), tz)
    rpw = sh.measure("total_rp", "Rp")
    sh.need(tw + rpw + 24 + sh.measure("total_label", d["words"]["total_gifts"]) + 48 <= x2 - mx, "total")
    base = total_y + 40 + sh.cap("total", tz) + 14
    sh.text("total_label", d["words"]["total_gifts"], mx, base - 8, id="total.label")
    sh.text("total", digits(total), x2, base, anchor="r", id="total")
    sh.text("total_rp", "Rp", x2 - tw - 24, base, anchor="r", id="total.rp")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "amount_w": round(amount_w(z), 1), "label_room": round(label_room(z), 1), "total_w": round(tw + rpw + 24, 1)})


def s04(sh, d, qr_on=True):
    g, k, s = sh.g, sh.k("s04"), d["s04"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["title"])
    card = k("card") if qr_on else 0
    x0 = mx
    wide = (x2 - mx - card - k("col_gap")) if qr_on else x2 - mx
    nsz = largest(k("number_max"), k("number_floor"), lambda z: sh.measure("number", s["number"], z) <= wide)
    hsz = largest(sh.size_of("holder"), k("holder_floor"), lambda z: sh.measure("holder", s["holder"], z) <= wide)
    sh.need(nsz is not None and hsz is not None, "account")
    y = k("stack_top")
    sh.text("bank", s["bank"], x0, y, id="bank")
    ncap = sh.cap("number", nsz)
    nb = y + 44 + ncap + 24
    sh.text("number", s["number"], x0, nb, size=nsz, id="number")
    sh.rect(x0, nb + 40, wide, 2, fill="ink", id="number.rule")
    sh.text("atas", d["words"]["holder"], x0, nb + 40 + 64, id="atas")
    sh.text("holder", s["holder"], x0, nb + 40 + 64 + 76, size=hsz, id="holder")
    ny = nb + 40 + 64 + 76 + 64
    for i, line in enumerate(s["note"]):
        for j, ln in enumerate(lines_for(sh, "note", line, wide, sh.size_of("note"))):
            sh.text("note", ln, x0, ny, id=f"note{i}.{j}")
            ny += 56
    if qr_on:
        qx, qy = x2 - card, 310
        qr(sh, qx, qy, card, "ink", "card", "paper", k("card_pad"))
        sh.text("caption", d["words"]["scan"], qx + card / 2, qy + card + 56, anchor="c", id="qr.caption")
    sh.fit.update({"number_size": nsz, "number_w": round(sh.measure("number", s["number"], nsz), 1), "wide": round(wide, 1), "bottom": round(ny, 1)})


def s05(sh, d):
    g, k, s = sh.g, sh.k("s05"), d["s05"]
    mx = g("mx")
    chrome(sh, d, s["date_long"])
    sh.text("caption", d["words"]["verse"], mx, k("ref_y") - 84, fill="accent", id="kicker2")
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
        sh.rect(x2 - k("photo_w"), 300, k("photo_w"), k("photo_h"), fill="rule", id="photo")
        sh.text("caption", "foto", x2 - k("photo_w") / 2, 300 + k("photo_h") / 2, anchor="c", id="photo.label")
    bsz = sh.size_of("n_body")
    body = [lines_for(sh, "n_body", p, width, bsz) for p in s["paragraphs"]]
    nb = sum(len(b) for b in body)

    def stack(z):
        _, lines = title_lines(sh, "n_title", s["title"], width, 2, z, z)
        return 380 + sh.cap("n_title", z) + (len(lines) - 1) * k("title_lh") * z + 56 + 4 + 64 + nb * 68 + len(body) * k("line_gap")

    z0, _ = title_lines(sh, "n_title", s["title"], width, 2, k("title_max"), k("title_floor"))
    z = largest(z0, k("title_floor"), lambda z: stack(z) <= g("body_bottom") + 40)
    sh.need(z is not None, "notice")
    z, lines = title_lines(sh, "n_title", s["title"], width, 2, z, z)
    y = 380
    sh.text("caption", d["words"]["notice"], mx, 332, fill="accent", id="kicker2")
    cap = sh.cap("n_title", z)
    for i, ln in enumerate(lines):
        sh.text("n_title", ln, mx, y + cap + i * k("title_lh") * z, size=z, id=f"title.{i}")
    y += cap + (len(lines) - 1) * k("title_lh") * z + 56
    sh.rect(mx, y, 96, 4, fill="accent", id="title.rule")
    y += 4 + 64
    for pi, bl in enumerate(body):
        for j, ln in enumerate(bl):
            sh.text("n_body", ln, mx, y, id=f"p{pi}.{j}")
            y += 68
        y += k("line_gap")
    sh.fit.update({"title_size": z, "title_lines": len(lines), "bottom": round(y, 1)})


LAYOUT = {"s01": s01, "s02": s02, "s03": s03, "s04": s04, "s05": s05, "s06": s06}
