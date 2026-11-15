"""Wireframes of four families (Serif italic d6, Neon night d3, Liturgy d11, Warm d17) with the season mark placed.

`elements_*` return plain lists of dicts (every number in them lands in spec.json); `serialize` turns a list into SVG and reads nothing else.
Text is measured with fontTools (no kerning). Placeholder rows are bars, not words.
"""
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fonts  # noqa: E402
from spec_data import FIXTURE  # noqa: E402

W, H = 1920, 1080


def r2(x):
    return round(float(x), 2)


class Wire:
    def __init__(self):
        self.el = []
        self.defs = []

    def rect(self, x, y, w, h, fill=None, rx=0, op=1, stroke=None, sw=0, rot=0, name=""):
        self.el.append(dict(t="rect", x=r2(x), y=r2(y), w=r2(w), h=r2(h), fill=fill, rx=r2(rx), op=op, stroke=stroke, sw=sw, rot=rot, rcx=r2(x + w / 2), rcy=r2(y + h / 2), name=name))

    def circle(self, cx, cy, r, fill=None, stroke=None, sw=0, op=1, name=""):
        self.el.append(dict(t="circle", cx=r2(cx), cy=r2(cy), r=r2(r), fill=fill, stroke=stroke, sw=sw, op=op, name=name))

    def path(self, d, fill=None, stroke=None, sw=0, op=1, name=""):
        self.el.append(dict(t="path", d=d, fill=fill, stroke=stroke, sw=sw, op=op, name=name))

    def text(self, x, y, s, file, family, weight, italic, size, ls_em, fill, anchor="start", op=1, name="", caps=False):
        s = s.upper() if caps else s
        w = fonts.advance(file, s, size, ls_em)
        self.el.append(dict(t="text", x=r2(x), y=r2(y), s=s, family=family, weight=weight, italic=italic, size=size, ls=r2(ls_em * size), fill=fill, anchor=anchor, op=op, name=name, w=r2(w)))
        return w

    def mark(self, glyph, x, y, size, color, op=1, name="mark"):
        self.el.append(dict(t="mark", glyph=glyph, x=r2(x), y=r2(y), size=size, color=color, op=op, name=name))

    def grad(self, gid, x1, y1, x2, y2, stops):
        self.defs.append(dict(id=gid, x1=x1, y1=y1, x2=x2, y2=y2, stops=stops))


def cap_cy(w, file, family, weight, italic, size, ls_em, cx, cy, s, fill, anchor="start", op=1, name="", caps=False):
    """Text whose cap height is centred on cy (the engine's `.cy()`)."""
    base = cy + fonts.cap_height(file, size) / 2
    return w.text(cx, base, s, file, family, weight, italic, size, ls_em, fill, anchor, op, name, caps)


# ---------------------------------------------------------------- d6 Serif italic
def d6(pal, glyph, grid, place):
    w = Wire()
    mx, x2 = grid["mx"], grid["x2"]
    w.rect(0, 0, W, H, pal["paper"], name="ground")
    w.grad("wash", 0, 0, 0, 1, [[0, pal["paper"], 0], [1, pal["wash"], 1]])
    w.rect(0, grid["wash_y"], W, grid["wash_h"], "url(#wash)", name="wash")
    kf, fam = "SourceSerif4Text-600.ttf", "Source Serif 4 Text"
    kw = w.text(mx, grid["kicker_y"], FIXTURE["kicker_word"], kf, fam, 600, False, 32, 0.16, pal["mute"], caps=True, name="kicker")
    w.text(x2, grid["kicker_y"], FIXTURE["church"], kf, fam, 600, False, 32, 0.16, pal["mute"], anchor="end", caps=True, name="kicker_right")
    w.rect(mx, grid["rule_y"], x2 - mx, grid["rule_w"], pal["ink"], name="rule")
    w.text(mx, grid["title_base"], FIXTURE["title"], "SourceSerif4Display-600.ttf", "Source Serif 4 Display", 600, False, 128, -0.01, pal["ink"], name="title")
    w.text(x2, grid["title_base"], FIXTURE["range"], "SourceSerif4Oldstyle-400i.ttf", "Source Serif 4 Oldstyle", 400, True, 52, 0, pal["mute"], anchor="end", name="range")
    schedule(w, pal, grid, "ink", "mute", "tint", "accent", 112, 336, 528, 60, 24, 332, "SourceSerif4Text-600.ttf", fam, 52)
    if glyph:
        cx = mx + kw + place["gap"]
        w.mark(glyph, cx, place["cy"] - place["size"] / 2, place["size"], pal[place["role"]], name="mark")
        w.mark_x = cx
    return w


def schedule(w, pal, grid, ink, mute, tint, accent, fig_x, time_x, ev_x, pitch, gap, top, tfile, tfam, tsize):
    groups = [("15", 2), ("18", 2), ("20", 2), ("21", 2)]
    y = top
    i = 0
    for gi, (day, n) in enumerate(groups):
        w.text(fig_x, y + 52, day, "SourceSerif4Display-400i.ttf", "Source Serif 4 Display", 400, True, 72, 0, pal[ink], name="day")
        for k in range(n):
            special = i == FIXTURE["special_row"]
            if special:
                w.rect(fig_x - 16, y - 4, 1712 + 16, pitch - 4, pal[tint], name="band")
                w.rect(fig_x - 16, y - 4, 6, pitch - 4, pal[accent], name="band_bar")
            w.text(time_x, y + 44, FIXTURE["rows"][i][0], tfile, tfam, 600, False, tsize, 0, pal[ink], name="time")
            w.rect(ev_x, y + 16, 1100 * FIXTURE["rows"][i][1], 24, pal[ink], rx=12, op=0.5, name="event")
            y += pitch
            i += 1
        y += gap


# ---------------------------------------------------------------- d3 Neon night
def d3(pal, glyph, grid, place, deco):
    w = Wire()
    mx = grid["mx"]
    w.grad("ground", 0, 0, 0, 1, [[0, pal["night1"], 1], [1, pal["night2"], 1]])
    w.rect(0, 0, W, H, "url(#ground)", name="ground")
    ring = None
    for kind, x, y, dia, colour in deco:
        if kind == "ring":
            cx, cy, r = x + dia / 2, y + dia / 2, dia / 2
            for off, gw, op in grid["glow"]:
                w.circle(cx, cy, r + off, None, pal[colour], gw, op, name="glow")
            w.circle(cx, cy, r - 5, None, pal[colour], 10, 1, name="ring")
            if ring is None and dia <= 200:
                ring = (cx, cy, dia, colour)
        elif kind == "diamond":
            for off, gw, op in grid["glow"]:
                w.rect(x - off, y - off, dia + 2 * off, dia + 2 * off, None, rx=10 + off, op=op, stroke=pal[colour], sw=gw, rot=45, name="glow")
            w.rect(x + 4, y + 4, dia - 8, dia - 8, None, rx=10, stroke=pal[colour], sw=8, rot=45, name="diamond")
        else:
            w.circle(x + dia / 2, y + dia / 2, dia / 2, pal[colour], name="dot")
    hf, hfam = "SpaceGrotesk-700.ttf", "Space Grotesk"
    kw = cap_cy(w, hf, hfam, 700, False, 32, 0.16, mx, grid["kicker_y"], "%s . %s" % (FIXTURE["kicker_word"], "Semua jam WIB"), pal["pink"], caps=True, name="kicker")
    w.text(mx, grid["title_base"], FIXTURE["title"], hf, hfam, 700, False, 100, 0, pal["ink"], caps=True, name="title")
    pill = FIXTURE["range_short"]
    pw = fonts.advance(hf, pill.upper(), 34, 0.08) + 60
    for off, gw, op in grid["glow"]:
        w.rect(mx - off, grid["pill_y"] - off, pw + 2 * off, grid["pill_h"] + 2 * off, None, rx=grid["pill_h"] / 2 + off, op=op, stroke=pal["pink"], sw=gw, name="glow")
    w.rect(mx, grid["pill_y"], pw, grid["pill_h"], None, rx=grid["pill_h"] / 2, stroke=pal["pink"], sw=4, name="pill")
    cap_cy(w, hf, hfam, 700, False, 34, 0.08, mx + 30, grid["pill_y"] + grid["pill_h"] / 2, pill, pal["pink"], caps=True, name="pill_text")
    top, pitch = 330, 80
    for i, (tm, frac) in enumerate(FIXTURE["rows"]):
        y = top + i * pitch
        special = i == FIXTURE["special_row"]
        if special:
            w.rect(mx - 24, y - 6, 1752 + 48, pitch - 12, None, rx=34, stroke=pal["yellow"], sw=4, name="highlight")
        w.rect(mx, y + 20, 110, 22, pal["pink"], rx=11, op=0.9, name="day")
        cap_cy(w, hf, hfam, 700, False, 46, 0, 296, y + 32, tm, pal["teal"], name="time")
        w.rect(456, y + 20, 1000 * frac, 22, pal["ink"], rx=11, op=0.55, name="event")
        if not special:
            w.rect(mx, y + pitch - 12, 1752, grid["line_w"], pal["line"], name="line")
    if glyph and ring:
        cx, cy, dia, colour = ring
        size = int(round(dia * place["ratio"]))
        w.mark(glyph, cx - size / 2, cy - size / 2, size, pal[colour], name="mark")
    return w


# ---------------------------------------------------------------- d11 Liturgy
def d11(pal, glyph, grid, place, cross=False):
    w = Wire()
    g = grid
    w.rect(0, 0, W, H, pal["ground2"], name="ground2")
    w.rect(g["frame"], g["frame"], W - 2 * g["frame"], H - 2 * g["frame"], pal["ground"], stroke=pal["gold"], sw=g["frame_sw"], name="frame")
    inner = g["frame"] + g["frame_gap"]
    w.rect(inner, inner, W - 2 * inner, H - 2 * inner, None, op=g["frame_op2"], stroke=pal["gold"], sw=g["frame_sw2"], name="frame2")
    for x, y in [(inner, inner), (inner, H - inner), (W - inner, inner), (W - inner, H - inner)]:
        w.rect(x - g["dia"] / 2, y - g["dia"] / 2, g["dia"], g["dia"], pal["gold"], rot=45, name="diamond")
    cx = W / 2
    w.rect(g["rule_x"], g["cross_cy"] - g["rule_w"] / 2, cx - g["rule_gap"] - g["rule_x"], g["rule_w"], pal["gold"], op=g["rule_op"], name="rule")
    w.rect(cx + g["rule_gap"], g["cross_cy"] - g["rule_w"] / 2, cx - g["rule_gap"] - g["rule_x"], g["rule_w"], pal["gold"], op=g["rule_op"], name="rule")
    if cross or not glyph:
        w.rect(cx - g["cross_w"] / 2, g["cross_cy"] - g["cross_h"] / 2, g["cross_w"], g["cross_h"], pal["gold"], name="cross")
        w.rect(cx - g["cross_arm"] / 2, g["cross_cy"] - g["cross_up"] - g["cross_arm_h"] / 2, g["cross_arm"], g["cross_arm_h"], pal["gold"], name="cross")
    cf, cfam = "Cinzel-600.ttf", "Cinzel"
    cap_cy(w, cf, cfam, 600, False, 32, 0.2, cx, g["kicker_cy"], "%s . %s" % (FIXTURE["kicker_word"], FIXTURE["church"]), pal["gold"], anchor="middle", caps=True, name="kicker")
    cap_cy(w, "Cinzel-700.ttf", cfam, 700, False, 88, 0.1, cx, g["title_cy"], FIXTURE["title"], pal["cream"], anchor="middle", caps=True, name="title")
    cap_cy(w, "SourceSerif4Text-400.ttf", "Source Serif 4 Text", 400, False, 40, 0.02, cx, g["sub_cy"], FIXTURE["range"], pal["mute"], anchor="middle", name="sub")
    top, pitch = 372, 74
    for i, (tm, frac) in enumerate(FIXTURE["rows"]):
        y = top + i * pitch
        special = i == FIXTURE["special_row"]
        if special:
            w.rect(g["pad"], y - 8, W - 2 * g["pad"], pitch - 8, pal["cream"], rx=g["band_rx"], name="band")
        w.rect(g["day_x"] if "day_x" in g else 96, y + 14, 120, 20, pal["gold"], rx=10, op=0.9, name="day")
        cap_cy(w, "SourceSerif4Text-600.ttf", "Source Serif 4 Text", 600, False, 48, 0, 330, y + 26, tm, pal["ink"] if special else pal["gold"], name="time")
        w.rect(520, y + 14, 1000 * frac, 22, pal["ink"] if special else pal["cream"], rx=11, op=0.6 if special else 0.55, name="event")
    if glyph and not cross:
        w.mark(glyph, cx - place["size"] / 2, place["cy"] - place["size"] / 2, place["size"], pal[place["role"]], name="mark")
    return w


# ---------------------------------------------------------------- d17 Hangat
def d17(pal, glyph, grid, place, arch):
    w = Wire()
    g = grid
    mx = g["mx"]
    w.rect(0, 0, W, H, pal["ground"], name="ground")
    floor = g["card_y"] + 14
    for x, y, aw, ah in arch["rows"]:
        h = min(ah, floor - y)
        rr = aw / 2
        d = "M%s %sV%sA%s %s 0 0 1 %s %sV%s" % (r2(x), r2(y + h), r2(y + rr), r2(rr), r2(rr), r2(x + aw), r2(y + rr), r2(y + h))
        w.path(d, None, pal["ink"], arch["sw"], arch["op"], name="arch")
    kf, kfam = "InstrumentSans-600.ttf", "Instrument Sans"
    kw = cap_cy(w, kf, kfam, 600, False, 32, 0.16, mx, g["kicker_cy"], "%s . %s" % (FIXTURE["kicker_word"], FIXTURE["church"]), pal["mute"], caps=True, name="kicker")
    tw = w.text(mx, g["title_base"], FIXTURE["title"], "Fraunces-600.ttf", "Fraunces", 600, False, 112, 0, pal["ink"], name="title")
    w.text(mx + tw + g["sub_gap"], g["title_base"], FIXTURE["range_short"], "Fraunces-400i.ttf", "Fraunces", 400, True, 56, 0, pal["accent"], name="sub")
    w.rect(mx, g["card_y"], W - 2 * mx, g["card_h"], pal["card"], rx=g["card_rx"], name="card")
    top, pitch = g["card_y"] + 28, 78
    for i, (tm, frac) in enumerate(FIXTURE["rows"]):
        y = top + i * pitch
        special = i == FIXTURE["special_row"]
        if special:
            w.rect(mx + 16, y - 4, W - 2 * mx - 32, pitch - 6, pal["accent"], rx=g["band_rx"] if "band_rx" in g else 20, name="highlight")
        cap_cy(w, kf, kfam, 600, False, 48, 0, mx + 56, y + 32, tm, pal["on_accent"] if special else pal["accent"], name="time")
        w.rect(mx + 300, y + 22, 1000 * frac, 22, pal["on_accent"] if special else pal["ink"], rx=11, op=0.7 if special else 0.5, name="event")
    if glyph:
        cx = mx + kw + place["gap"]
        w.mark(glyph, cx, place["cy"] - place["size"] / 2, place["size"], pal[place["role"]], name="mark")
    return w


# ---------------------------------------------------------------- serialise
def serialize(w, glyph_bodies, spec_view=24, width=W, height=H):
    out = ['<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">' % (width, height, width, height)]
    if w.defs:
        out.append("<defs>")
        for d in w.defs:
            out.append('<linearGradient id="%s" x1="%s" y1="%s" x2="%s" y2="%s">' % (d["id"], d["x1"], d["y1"], d["x2"], d["y2"]))
            for off, col, op in d["stops"]:
                out.append('<stop offset="%s" stop-color="%s" stop-opacity="%s"/>' % (off, col, op))
            out.append("</linearGradient>")
        out.append("</defs>")
    for e in w.el:
        t = e["t"]
        paint = ""
        if t in ("rect", "circle", "path"):
            paint += ' fill="%s"' % (e["fill"] if e["fill"] else "none")
            if e["stroke"]:
                paint += ' stroke="%s" stroke-width="%s"' % (e["stroke"], e["sw"])
            if e["op"] != 1:
                paint += ' opacity="%s"' % e["op"]
        if t == "rect":
            tr = ""
            if e["rot"]:
                tr = ' transform="rotate(%s %s %s)"' % (e["rot"], e["rcx"], e["rcy"])
            rx = ' rx="%s"' % e["rx"] if e["rx"] else ""
            out.append('<rect x="%s" y="%s" width="%s" height="%s"%s%s%s/>' % (e["x"], e["y"], e["w"], e["h"], rx, paint, tr))
        elif t == "circle":
            out.append('<circle cx="%s" cy="%s" r="%s"%s/>' % (e["cx"], e["cy"], e["r"], paint))
        elif t == "path":
            out.append('<path d="%s"%s/>' % (e["d"], paint))
        elif t == "text":
            style = ' font-style="italic"' if e["italic"] else ""
            ls = ' letter-spacing="%s"' % e["ls"] if e["ls"] else ""
            op = ' opacity="%s"' % e["op"] if e["op"] != 1 else ""
            s = e["s"].replace("&", "&amp;")
            out.append('<text x="%s" y="%s" font-family="%s" font-weight="%s"%s font-size="%s"%s text-anchor="%s" fill="%s"%s>%s</text>' % (e["x"], e["y"], e["family"], e["weight"], style, e["size"], ls, e["anchor"], e["fill"], op, s))
        elif t == "mark":
            body = glyph_bodies[e["glyph"]].replace("currentColor", e["color"])
            op = ' opacity="%s"' % e["op"] if e["op"] != 1 else ""
            out.append('<svg x="%s" y="%s" width="%s" height="%s" viewBox="0 0 %d %d"%s>%s</svg>' % (e["x"], e["y"], e["size"], e["size"], spec_view, spec_view, op, body))
    out.append("</svg>")
    return "\n".join(out)
