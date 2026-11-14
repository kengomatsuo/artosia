"""Candidate D, Civic Board: the six kinds. A header band with the title in mono caps, mono figures, condensed words, inverted rows."""
from engine import Sheet, Split, digits, largest, W, H
from kit import draw_verse, fit_events, groups, hl_runs, lines_for, name_grid, qr, title_lines, verse_lines


def chrome(sh, d, title_text, right=None, sub=None):
    g = sh.g
    sh.rect(0, 0, W, H, fill="paper", id="ground")
    sh.rect(0, 0, W, g("band_h"), fill="band", id="band")
    sh.text("kicker", d["words"]["warta"], g("mx"), g("kicker_y"), id="kicker")
    sh.text("kicker", right or d["church"], g("x2"), g("kicker_y"), anchor="r", id="kicker.r")
    t = sh.text("title", title_text, g("mx"), g("title_base"), id="title")
    s = None
    if sub:
        s = sh.text("subtitle", sub, g("x2"), g("title_base"), anchor="r", id="subtitle")
        sh.need(s["x"] > t["x"] + t["w"] + 32, "header band")
    return t, s


def foot(sh, part):
    if part:
        sh.text("foot", part, sh.g("mx"), 1050, fill="ink", id="foot")


def s01(sh, d, part=None):
    g, k, s = sh.g, sh.k("s01"), d["s01"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["title"], sub=s["week_short"])
    rows, gs = s["rows"], groups(s["rows"])
    top, bottom = g("body_top"), g("body_bottom")
    ev_w = lambda r: x2 - k("event_x") - (k("tag_room") if r["special"] else 24)

    def height(z, extra=0.0):
        return sum(len(r) * (k("ratio") * z + extra) for _, r in gs) + (len(gs) - 1) * k("group_gap")

    got = fit_events(sh, rows, "event", "event_hl", ev_w, k("size_max"), k("size_floor"), lambda z, e: height(z, e) <= bottom - top)
    sh.need(got is not None, "schedule rows")
    z, lines, extra = got
    pitch = k("ratio") * z + extra
    cap = sh.cap("event", z)
    mz = sh.size_of("time")
    y, ri, texts = top, 0, []
    for gi, (i0, grp) in enumerate(gs):
        for j, r in enumerate(grp):
            ry = y + j * pitch
            if r["special"]:
                sh.rect(mx - g("band_pad"), ry, x2 - mx + 2 * g("band_pad"), pitch, fill="band", id=f"g{gi}.r{j}.inv")
            else:
                sh.rect(mx, ry + pitch - 0.75, x2 - mx, g("hair"), fill="rule", id=f"g{gi}.r{j}.rule")
        texts.append((gi, y, grp, ri))
        ri += len(grp)
        y += len(grp) * pitch + k("group_gap")
    for gi, y, grp, ri in texts:
        for j, r in enumerate(grp):
            ry = y + j * pitch
            ev = lines[ri + j]
            mid = ry + pitch / 2
            hl = r["special"]
            if j == 0:
                sh.text("day_name", f"{grp[0]['dow']} {grp[0]['date']}", k("day_x"), mid + sh.cap("day_name") / 2, fill="band_ink" if hl else None, id=f"g{gi}.day")
            sh.text("time_hl" if hl else "time", r["time"], k("time_x"), mid + sh.cap("time") / 2, id=f"g{gi}.r{j}.time")
            for li, ln in enumerate(ev):
                sh.text("event_hl" if hl else "event", ln, k("event_x"), mid + (li - (len(ev) - 1) / 2) * 1.12 * z + cap / 2, size=z, id=f"g{gi}.r{j}.event{li}")
            if hl:
                sh.text("tag", d["words"]["special"], x2, mid + sh.cap("tag") / 2, anchor="r", id=f"g{gi}.r{j}.tag")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "height": round(height(z, extra), 1), "room": bottom - top, "two_line_rows": sum(len(l) > 1 for l in lines)})
    foot(sh, part)


def s02(sh, d, key="s02", part=None, caption=None):
    g, k, s = sh.g, sh.k("s02"), d[key]
    mx, x2 = g("mx"), g("x2")
    names = s["names"]
    chrome(sh, d, s["title"], sub=s["week_label"])
    cols = next(c for lim, c in k("cols") if len(names) <= lim)
    spans = s["spans"]
    mon_w = max((sh.measure("bday_mon", n["month"]) for n in names), default=0) if spans else 0
    day_cells = lambda z: sh.measure("bday_day_hl", "00", z) + ((k("day_gap") + mon_w) if spans else 0)
    cells, z, pitch, colw = name_grid(sh, names, cols, g("body_top"), g("body_bottom"), mx, x2, k("col_gap"), "name", "name_hl", k("size_max"), k("size_floor"), k("ratio"), day_cells, k("name_gap"))
    per = -(-len(names) // cols)
    num_w = sh.measure("bday_day_hl", "00", z)
    for cl in cells:
        sh.rect(cl["cx"], cl["cy"] + pitch / 2 - 0.75, colw, g("hair"), fill="rule", id=f"n{cl['i']}.rule")
    for c, r0, r1 in hl_runs(names, cells, per):
        cx = mx + c * (colw + k("col_gap"))
        sh.rect(cx - k("band_over"), g("body_top") + r0 * pitch, colw + 2 * k("band_over"), (r1 - r0 + 1) * pitch, fill="band", id=f"inv.c{c}")
    cap = sh.cap("name", z)
    for cl in cells:
        nm = names[cl["i"]]
        base = cl["cy"] + cap / 2
        hl = nm["hl"]
        sh.text("bday_day_hl" if hl else "bday_day", f"{nm['day']:02d}", cl["cx"] + num_w, base, anchor="r", size=z - 4, id=f"n{cl['i']}.day")
        if spans:
            sh.text("bday_mon_hl" if hl else "bday_mon", nm["month"], cl["cx"] + num_w + k("day_gap"), base, id=f"n{cl['i']}.mon")
        sh.text("name_hl" if hl else "name", cl["text"], cl["cx"] + day_cells(z) + k("name_gap"), base, size=z, id=f"n{cl['i']}.name")
    sh.text("caption", s["month"], mx, 1050, id="month")
    if part:
        sh.text("caption", part + ("   " + caption if caption else ""), x2, 1050, anchor="r", id="foot")


def s03(sh, d):
    g, k, s = sh.g, sh.k("s03"), d["s03"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["title"], right=d["words"]["offerings"], sub=s["date_long"])
    funds = s["funds"]
    n = len(funds)
    top = g("body_top")
    total_y = g("body_bottom") - k("total_h")
    avail = total_y - k("total_gap") - top
    rp_w = sh.measure("rp", "Rp")
    amount_w = lambda z: max(sh.measure("amount", digits(f["rupiah"]), z - 6) for f in funds)
    label_room = lambda z: x2 - mx - rp_w - k("rp_gap") - amount_w(z) - 48
    z = largest(k("size_max"), k("size_floor"), lambda z: n * k("ratio") * z <= avail and all(sh.measure("fund", f["label"], z) <= label_room(z) for f in funds))
    sh.need(z is not None, "fund rows")
    pitch = min(avail / n, 1.9 * z)
    cap = sh.cap("fund", z)
    for i, f in enumerate(funds):
        base = top + i * pitch + (pitch + cap) / 2
        sh.rect(mx, top + (i + 1) * pitch - 0.75, x2 - mx, g("hair"), fill="rule", id=f"f{i}.rule")
        sh.text("fund", f["label"], mx, base, size=z, id=f"f{i}.label")
        sh.text("amount", digits(f["rupiah"]), x2, base, anchor="r", size=z - 6, id=f"f{i}.amount")
        sh.text("rp", "Rp", x2 - amount_w(z) - k("rp_gap"), base, anchor="r", id=f"f{i}.rp")
    sh.rect(mx - g("band_pad"), total_y, x2 - mx + 2 * g("band_pad"), k("total_h"), fill="band", id="total.inv")
    total = sum(f["rupiah"] for f in funds)
    rpw = sh.measure("total_rp", "Rp")
    tz = largest(sh.size_of("total"), 64, lambda z: sh.measure("total", digits(total), z) + rpw + 20 <= x2 - mx - 8)
    sh.need(tz is not None, "total")
    tw = sh.measure("total", digits(total), tz)
    sh.text("total_label", d["words"]["total_gifts"], mx, total_y + 52, id="total.label")
    sh.text("total", digits(total), x2, total_y + k("total_h") - 36, anchor="r", size=tz, id="total")
    sh.text("total_rp", "Rp", x2 - tw - 20, total_y + k("total_h") - 36, anchor="r", id="total.rp")
    sh.fit.update({"size": z, "pitch": round(pitch, 2), "amount_w": round(amount_w(z), 1), "label_room": round(label_room(z), 1), "total_w": round(tw + rpw + 20, 1), "total_size": tz})


def s04(sh, d, qr_on=True):
    g, k, s = sh.g, sh.k("s04"), d["s04"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, s["title"], right=d["words"]["account"])
    card = k("card") if qr_on else 0
    wide = (x2 - mx - card - k("col_gap")) if qr_on else x2 - mx
    nsz = largest(k("number_max"), k("number_floor"), lambda z: sh.measure("number", s["number"], z) <= wide)
    hsz = largest(sh.size_of("holder"), k("holder_floor"), lambda z: sh.measure("holder", s["holder"], z) <= wide)
    sh.need(nsz is not None and hsz is not None, "account")
    y = k("stack_top")
    sh.text("bank", s["bank"], mx, y, id="bank")
    nb = y + 34 + 0.7 * nsz
    sh.text("number", s["number"], mx, nb, size=nsz, id="number")
    sh.rect(mx, nb + 36, wide, 3, fill="band", id="number.rule")
    sh.text("atas", d["words"]["holder"], mx, nb + 36 + 62, id="atas")
    sh.text("holder", s["holder"], mx, nb + 36 + 62 + 72, size=hsz, id="holder")
    ny = nb + 36 + 62 + 72 + 64
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
    chrome(sh, d, d["words"]["verse"], right=s["date_long"])
    sh.text("ref", s["reference"], mx, k("ref_y"), id="ref")
    room = k("bottom") - k("top")
    z = largest(k("size_max"), k("size_floor"), lambda z: len(verse_lines(sh, s["verses"], k("text_w"), z, k("sup_scale"))) * k("lh") * z <= room)
    sh.need(z is not None, "verse")
    lines = verse_lines(sh, s["verses"], k("text_w"), z, k("sup_scale"))
    draw_verse(sh, lines, mx, k("top"), z, k("lh"), "verse", "verse_n", "ink", k("sup_scale"), k("sup_rise"), id="verse")
    sh.fit.update({"size": z, "lines": len(lines), "height": round(len(lines) * k("lh") * z, 1), "room": room})


def s06(sh, d, photo=True):
    g, k, s = sh.g, sh.k("s06"), d["s06"]
    mx, x2 = g("mx"), g("x2")
    chrome(sh, d, d["words"]["notice"], right=s["date_long"])
    width = (x2 - mx - k("photo_w") - k("photo_gap")) if photo else 1560
    if photo:
        sh.rect(x2 - k("photo_w"), 280, k("photo_w"), k("photo_h"), fill="rule", id="photo")
        sh.text("caption", "foto", x2 - k("photo_w") / 2, 280 + k("photo_h") / 2, anchor="c", id="photo.label")
    bsz = sh.size_of("n_body")
    body = [lines_for(sh, "n_body", p, width, bsz) for p in s["paragraphs"]]
    nb = sum(len(b) for b in body)
    top = 290

    def stack(z):
        _, lines = title_lines(sh, "n_title", s["title"], width, 2, z, z, upper=True)
        return top + 0.7 * z + (len(lines) - 1) * k("title_lh") * z + 130 + nb * 68 + len(body) * k("line_gap")

    z0, _ = title_lines(sh, "n_title", s["title"], width, 2, k("title_max"), k("title_floor"), upper=True)
    z = largest(z0, k("title_floor"), lambda z: stack(z) <= g("body_bottom"))
    sh.need(z is not None, "notice")
    z, lines = title_lines(sh, "n_title", s["title"], width, 2, z, z, upper=True)
    y = top
    for i, ln in enumerate(lines):
        sh.text("n_title", ln, mx, y + 0.7 * z + i * k("title_lh") * z, size=z, id=f"title.{i}")
    y += 0.7 * z + (len(lines) - 1) * k("title_lh") * z + 48
    sh.rect(mx, y, 144, 6, fill="band", id="title.rule")
    y += 6 + 76
    for pi, bl in enumerate(body):
        for j, ln in enumerate(bl):
            sh.text("n_body", ln, mx, y, id=f"p{pi}.{j}")
            y += 68
        y += k("line_gap")
    sh.fit.update({"title_size": z, "title_lines": len(lines), "bottom": round(y, 1)})


LAYOUT = {"s01": s01, "s02": s02, "s03": s03, "s04": s04, "s05": s05, "s06": s06}
