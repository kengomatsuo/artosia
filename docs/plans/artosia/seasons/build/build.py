"""SEASONMARK1 build: spec.json, the marks, the wireframes, the sheets and the colourway files.

python -I build.py spec | marks | wire | sheets | docs | all
Reads the hand-written data (spec_data.py, glyphs.py, palettes.py), writes spec.json, and every drawing is serialised from spec.json alone.
Renderer: resvg-js 2.6.2-beta.1 through `bunx @resvg/resvg-js-cli` (system fonts off, the bundled faces from crates/artosia-slides/assets/warta/fonts).
"""
import json
import math
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fonts  # noqa: E402
import glyphs  # noqa: E402
import palettes  # noqa: E402
import spec_data  # noqa: E402
import wire  # noqa: E402
from colour import contrast, delta_e, to_oklch  # noqa: E402

ROOT = os.path.normpath(os.path.join(HERE, ".."))
SPEC = os.path.join(ROOT, "spec.json")
W, H = 1920, 1080
SEASON_NAMES = {"advent": "Advent", "christmas": "Christmas", "lent": "Lent", "easter": "Easter", "pentecost": "Pentecost"}
FAMILY_SHORT = {1: "Editorial greige", 2: "Pill poster", 3: "Neon night", 4: "Split panel", 5: "Simple", 6: "Serif italic", 7: "Swiss numerals", 8: "Quiet Tint", 9: "Civic Board", 10: "Diagonal", 11: "Liturgy", 12: "Glass", 13: "Block", 14: "Cahaya", 15: "Magazine", 16: "Figures", 17: "Warm"}
WIRE_FAMILIES = [(6, "Serif italic"), (3, "Neon night"), (11, "Liturgy"), (17, "Warm")]
WIRE_SEASONS = ["christmas", "easter"]
SPOT_SHORT = {6: "the mark follows the kicker word on the top line", 3: "the mark sits inside the first neon ring", 11: "the mark takes the place of the small cross", 17: "the mark follows the kicker on the top line"}
GLYPH_FILE = {k: k + ".svg" for k in glyphs.ORDER}
LIGHT_TILE, LIGHT_INK, DARK_TILE, DARK_INK = "#F4F1EA", "#1D1A15", "#16181D", "#EEEEEE"


def load_dir(n):
    return json.load(open(os.path.join(palettes.DIRS, "d%d.json" % n), encoding="utf-8"))


def hue_gap(a, b):
    la, lb = to_oklch(a), to_oklch(b)
    if la[1] < 0.03 or lb[1] < 0.03:
        return 0
    d = abs(la[2] - lb[2])
    return min(d, 360 - d)


def retag_of(d, pal):
    """An existing untagged colourway whose swatch is almost the new one: tag it instead of adding a near copy."""
    best = None
    for i, c in enumerate(d["colourways"]):
        if c.get("season") or i == 0:
            continue
        dist = max(delta_e(pal[k], c["palette"][k]) for k in d["swatch"])
        gap = max(hue_gap(pal[k], c["palette"][k]) for k in d["swatch"])
        if best is None or dist < best[1]:
            best = (c["id"], dist, gap)
    if best and best[1] < 0.05 and best[2] < 35:
        return best[0]
    return None


def make_spec():
    g = glyphs.make()
    gl = {}
    for k in glyphs.ORDER:
        title, season, body = g[k]
        gl[k] = dict(title=title, season=season, file="marks/" + GLYPH_FILE[k], body=body)
    res = palettes.generate()
    fams = {}
    for n in range(1, 18):
        d = load_dir(n)
        r = res[n]
        pl = spec_data.PLACEMENT[n]
        pairs = d["pairs"]
        # the mark pair: the role the mark takes on the ground it sits on, and its lowest ratio over every colourway of the family
        roles = [pl["role"]] if pl["spot"] in ("kicker", "sticker", "panel", "cross", "corner") else (["pink", "yellow", "teal"] if pl["spot"] == "ring" else [])
        mark_pairs = []
        for role in roles:
            ground = pl["ground"]
            kind = next((k for a, b, k in pairs if a == role and b == ground), None)
            lowest = min(contrast(c["palette"][role], c["palette"][ground]) for c in d["colourways"] + [dict(palette=x["palette"]) for x in r["new"]])
            mark_pairs.append(dict(role=role, ground=ground, listed_as=kind, lowest_ratio=round(lowest, 2)))
        new = []
        for x in r["new"]:
            new.append(dict(id=x["id"], season=x["season"], mode=x["mode"], palette=x["palette"], ratios=x["ratios"], min_ratio=min(x["ratios"]), nearest=x["nearest"], nearest_de=x["nearest_de"], retag=retag_of(d, x["palette"])))
        fams[str(n)] = dict(name=d["name"], short=FAMILY_SHORT[n], folder=d.get("folder"), swatch=d["swatch"], keys=r["keys"], pairs=pairs, have=r["have"], existing=[dict(id=c["id"], season=c.get("season")) for c in d["colourways"]], placement=pl, mark_pairs=mark_pairs, new=new, grid={k: v for k, v in d["grid"].items() if not isinstance(v, list)}, glow=d["grid"].get("glow"))
    spec = dict(
        meta=dict(card="SEASONMARK1", date="2026-11-15", renderer="resvg-js 2.6.2-beta.1 via bunx @resvg/resvg-js-cli, system fonts off, fonts from crates/artosia-slides/assets/warta/fonts", text_measure="fontTools advance widths from the same faces, tracking added, no kerning",
                   contrast="WCAG 2 relative luminance with the 0.03928 threshold, as crates/artosia-slides/src/look.rs contrast_of; gates text 4.5, large 3, mark 3; the solver keeps 0.05 above the gate"),
        grid=dict(view=24, stroke=2, cap="round", join="round", solid_below_units=5, live=[2, 22], sizes_px=[48, 64, 96, 160], min_px=48, max_px=160, weights_tested=[1.5, 2, 2.5], sheet=[W, H]),
        glyphs=gl,
        season_mark=spec_data.SEASON_MARK,
        by_hand_only=spec_data.BY_HAND_ONLY,
        advent_week_rule=spec_data.ADVENT_WEEK_RULE,
        fixture=spec_data.FIXTURE,
        paint=palettes.PAINT,
        d3_neon=palettes.D3_NEON,
        sheet_colours=["#FFFFFF", "#000000", "#EDEDED", LIGHT_TILE, LIGHT_INK, DARK_TILE, DARK_INK, "#5A564E", "#E0201B"],
        families=fams,
    )
    spec["wireframes"] = make_wireframes(spec)
    spec["compare"] = make_compare(spec)
    spec["sheets"] = make_sheets(spec)
    json.dump(spec, open(SPEC, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    return spec


# ------------------------------------------------------------------ wireframes
def palette_of(spec, n, season):
    f = spec["families"][str(n)]
    for x in f["new"]:
        if x["season"] == season:
            return x["palette"], x["id"], "new"
    d = load_dir(n)
    for c in d["colourways"]:
        if c.get("season") == season:
            return c["palette"], c["id"], "existing"
    raise KeyError((n, season))


def glyph_of(season):
    return {"christmas": "christmas", "easter": "easter", "advent": "advent-1", "lent": "lent", "pentecost": "pentecost"}[season]


def els(w):
    return dict(el=w.el, defs=w.defs)


def overlaps(el, m):
    """Names of the drawn elements (text, bars, rules) a mark's rectangle covers."""
    mb = (m["x"], m["y"], m["x"] + m["size"], m["y"] + m["size"])
    hit = []
    for e in el:
        if e["t"] not in ("text", "rect") or e.get("name") in ("ground", "ground2", "wash", "frame", "frame2", "card", "band", "diamond", "glow", "ring", "pill", "highlight", "line", "arch", "mark"):
            continue
        if e["t"] == "text":
            wd = e["w"]
            x = e["x"] - (wd if e["anchor"] == "end" else (wd / 2 if e["anchor"] == "middle" else 0))
            bb = (x, e["y"] - e["size"] * 0.8, x + wd, e["y"] + e["size"] * 0.2)
        else:
            bb = (e["x"], e["y"], e["x"] + e["w"], e["y"] + e["h"])
        if bb[0] < mb[2] and mb[0] < bb[2] and bb[1] < mb[3] and mb[1] < bb[3]:
            hit.append(e.get("name") or e["t"])
    return sorted(set(hit))


def make_wireframes(spec):
    out = {}
    for n, short in WIRE_FAMILIES:
        d = load_dir(n)
        grid = d["grid"]
        pl = spec_data.PLACEMENT[n]
        for season in WIRE_SEASONS:
            pal, cid, kind = palette_of(spec, n, season)
            gname = glyph_of(season)
            key = "%s-%s" % (short.lower().replace(" ", "-"), season)
            if n == 6:
                w = wire.d6(pal, gname, grid, pl)
            elif n == 3:
                w = wire.d3(pal, gname, grid, pl, d["deco"]["s01"])
            elif n == 11:
                w = wire.d11(pal, gname, grid, pl)
            else:
                w = wire.d17(pal, gname, grid, pl, d["arch"])
            mark = next(e for e in w.el if e["t"] == "mark")
            ground = pal[{6: "paper", 3: "night1", 11: "ground", 17: "ground"}[n]]
            out[key] = dict(family=n, season=season, colourway=cid, colourway_kind=kind, glyph=gname, mark=mark, ground=ground, tone="light" if sum(int(ground[i:i + 2], 16) for i in (1, 3, 5)) > 380 else "dark", **els(w))
            if (n, season) in ((6, "christmas"), (11, "easter")):
                # design B: no sign beside the kicker, a 160 px emblem in the top right corner (the church packs' corner motif)
                wb = wire.Wire()
                wb = wire.d6(pal, None, grid, pl) if n == 6 else wire.d11(pal, None, grid, pl)
                x2 = grid["x2"] if "x2" in grid else wire.W - grid["pad"]
                role = "accent" if n == 6 else "gold"
                wb.mark(gname, x2 - 160, 40, 160, pal[role], name="mark")
                mb = next(e for e in wb.el if e["t"] == "mark")
                out[key + "-B"] = dict(family=n, season=season, colourway=cid, colourway_kind=kind, glyph=gname, mark=mb, ground=ground, tone=out[key]["tone"], overlaps=overlaps(wb.el, mb), **els(wb))
    return out


# ------------------------------------------------------------------ design A against design B
def make_compare(spec):
    """A: the 48 px sign beside the kicker. B: a 160 px emblem in the top right or bottom right corner, as the church packs do (notes.md rows 1, 2, 5, 21)."""
    rows = []
    sheet = W * H
    for n in range(1, 18):
        f = spec["families"][str(n)]
        g = f["grid"]
        x2 = g.get("x2", W - g.get("mx", 56))
        bottom = g.get("body_bottom") or g.get("body_bot") or g.get("rows_bot") or 1000
        rows.append(dict(family=n, x2=x2, top_right_holds=spec_data.TOP_RIGHT[n], top_right_free=n in spec_data.TOP_RIGHT_FREE, body_bottom=bottom, bottom_right_in_body=bottom > H - 40 - 160))
    area_a, area_b = 48 * 48, 160 * 160
    return dict(rows=rows, area_a_pct=round(100 * area_a / sheet, 3), area_b_pct=round(100 * area_b / sheet, 3), emblem_px=160, sign_px=48,
                top_hits=sum(not r["top_right_free"] for r in rows), bottom_hits=sum(r["bottom_right_in_body"] for r in rows), free=len(spec_data.TOP_RIGHT_FREE))


# ------------------------------------------------------------------ sheets (lists of primitives)
def sheet_text(w, x, y, s, size, fill, weight=600, anchor="start", family="Instrument Sans", file="InstrumentSans-600.ttf"):
    w.text(x, y, s, file, family, weight, False, size, 0, fill, anchor)


def make_sheets(spec):
    sheets = {}
    # ---- contact sheet
    w = wire.Wire()
    SW = 2480
    w.rect(0, 0, SW, 4060, "#FFFFFF", name="paper")
    sheet_text(w, 40, 70, "Season marks and four families in two seasons", 44, "#1D1A15")
    sheet_text(w, 40, 116, "SEASONMARK1. Sixteen marks, currentColor only, drawn here at 160 px. Wireframes at 1920 x 1080 from spec.json; each red box is the mark's rectangle.", 28, "#5A564E", 500, file="InstrumentSans-500.ttf")
    sheet_text(w, 40, 156, "Rendered by resvg-js (bunx @resvg/resvg-js-cli, bundled faces); text widths measured with fontTools, so a kicker's end is within a few percent of the engine's.", 28, "#5A564E", 500, file="InstrumentSans-500.ttf")
    tile_w, tile_h = 300, 250
    for i, name in enumerate(glyphs.ORDER):
        x, y = 40 + (i % 8) * tile_w, 190 + (i // 8) * tile_h
        w.rect(x, y, tile_w - 20, tile_h - 20, LIGHT_TILE, rx=12, name="tile")
        w.mark(name, x + (tile_w - 20 - 160) / 2, y + 14, 160, LIGHT_INK, name="glyph")
        sheet_text(w, x + (tile_w - 20) / 2, y + 205, name, 26, "#5A564E", 500, "middle", file="InstrumentSans-500.ttf")
    y0 = 190 + 2 * tile_h + 50
    scale = 0.6146
    fw, fh = int(W * scale), int(H * scale)
    row_h = fh + 150
    sheets["contact"] = dict(width=SW, height=4060, items=w.el, frames=[])
    frames = []
    for r, (n, short) in enumerate(WIRE_FAMILIES):
        pl = spec_data.PLACEMENT[n]
        yy = y0 + r * row_h
        sheet_text(w, 40, yy + 34, "%s (d%d): %s" % (short, n, SPOT_SHORT[n]), 30, "#1D1A15")
        for c, season in enumerate(WIRE_SEASONS):
            key = "%s-%s" % (short.lower().replace(" ", "-"), season)
            wf = spec["wireframes"][key]
            x = 40 + c * (fw + 60)
            frames.append(dict(x=x, y=yy + 56, w=fw, h=fh, vw=W, vh=H, key=key))
            m = wf["mark"]
            # a red box round the mark, in sheet coordinates
            bx, by = x + m["x"] * scale, yy + 56 + m["y"] * scale
            w.rect(bx - 4, by - 4, m["size"] * scale + 8, m["size"] * scale + 8, None, stroke="#E0201B", sw=2, name="callout")
            label = "%s . colourway %s (%s, %s) . mark %s . %d px . %s" % (SEASON_NAMES[season], wf["colourway"], wf["colourway_kind"], wf["tone"], wf["glyph"], m["size"], "role " + pl["role"] if n != 3 else "the ring's colour")
            sheet_text(w, x, yy + 56 + fh + 36, label, 26, "#5A564E", 500, file="InstrumentSans-500.ttf")
    sheets["contact"]["frames"] = frames
    sheets["contact"]["items"] = w.el
    # ---- sizes and grounds
    w2 = wire.Wire()
    cols = [48, 64, 96, 160]
    SW2, SH2 = 3060, 1700
    w2.rect(0, 0, SW2, SH2, "#FFFFFF", name="paper")
    sheet_text(w2, 40, 60, "The sixteen marks at 48, 64, 96 and 160 px on a light and a dark ground", 40, "#1D1A15")
    for i, name in enumerate(glyphs.ORDER):
        blk, row = i // 8, i % 8
        bx = 40 + blk * 1420
        by = 100 + row * 170
        sheet_text(w2, bx, by + 90, name, 26, "#1D1A15", 600)
        x = bx + 200
        for tone, (tile, ink) in enumerate(((LIGHT_TILE, LIGHT_INK), (DARK_TILE, DARK_INK))):
            w2.rect(x - 14, by, 4 * 0 + 590, 160, tile, rx=10, name="tile")
            xx = x
            for s in cols:
                w2.mark(name, xx, by + (160 - s) / 2, s, ink, name="glyph")
                xx += s + 30
            x += 600
    wy = 100 + 8 * 170 + 20
    sheet_text(w2, 40, wy + 34, "Stroke weight test at 48 px: 1.5, 2 (chosen) and 2.5 units, light and dark", 30, "#1D1A15")
    wx = 40
    for gname in ("advent-3", "good-friday", "pentecost", "reformation", "easter", "palm"):
        for wgt in spec["grid"]["weights_tested"]:
            for tile, ink in ((LIGHT_TILE, LIGHT_INK), (DARK_TILE, DARK_INK)):
                w2.rect(wx - 6, wy + 60, 72, 72, tile, rx=8, name="tile")
                w2.mark("%s@%s" % (gname, wgt), wx, wy + 72, 48, ink, name="weight")
                wx += 80
        wx += 20
    sheets["sizes"] = dict(width=SW2, height=SH2, items=w2.el, frames=[])
    # ---- colourway chips
    w3 = wire.Wire()
    cw, ch = 470, 96
    SW3, SH3 = 90 + 5 * cw, 90 + 17 * ch + 40
    w3.rect(0, 0, SW3, SH3, "#EDEDED", name="paper")
    for i, s in enumerate(palettes.SEASONS):
        sheet_text(w3, 90 + i * cw + 6, 44, SEASON_NAMES[s], 28, "#1D1A15")
    for n in range(1, 18):
        f = spec["families"][str(n)]
        y = 70 + (n - 1) * ch
        sheet_text(w3, 6, y + 40, "d%d" % n, 26, "#1D1A15")
        sheet_text(w3, 6, y + 70, f["short"][:10], 18, "#5A564E", 500, file="InstrumentSans-500.ttf")
        d = load_dir(n)
        for i, s in enumerate(palettes.SEASONS):
            x = 90 + i * cw
            pal, tag, cid = None, "", ""
            for c in d["colourways"]:
                if c.get("season") == s:
                    pal, tag, cid = c["palette"], "existing", c["id"]
            for nw in f["new"]:
                if nw["season"] == s:
                    pal, tag, cid = nw["palette"], "retag " + nw["retag"] if nw["retag"] else "new", nw["id"]
            keys = list(pal.keys())
            w3.rect(x, y, cw - 10, ch - 10, pal[keys[0]], name="cell", stroke="#000000" if tag == "existing" else None, sw=4 if tag == "existing" else 0)
            sz = (cw - 30) / len(keys)
            for j, k in enumerate(keys):
                w3.rect(x + 10 + j * sz, y + 10, sz - 4, 44, pal[k], name="chip")
            lum = sum(int(pal[keys[0]][i:i + 2], 16) for i in (1, 3, 5))
            sheet_text(w3, x + 10, y + 78, "%s . %s" % (cid, tag), 18, "#FFFFFF" if lum < 380 else "#1D1A15", 600)
    sheets["chips"] = dict(width=SW3, height=SH3, items=w3.el, frames=[])
    # ---- design A against design B
    w4 = wire.Wire()
    SW4 = 2480
    scale = 0.6146
    fw, fh = int(W * scale), int(H * scale)
    SH4 = 150 + 2 * (fh + 170) + 330
    w4.rect(0, 0, SW4, SH4, "#FFFFFF", name="paper")
    sheet_text(w4, 40, 70, "Two designs for marking a season, drawn on the same two cards", 44, "#1D1A15")
    sheet_text(w4, 40, 116, "A: a 48 px sign beside the kicker. B: a 160 px emblem in the top right corner, as the church packs do. Red boxes are the mark's rectangle.", 28, "#5A564E", 500, file="InstrumentSans-500.ttf")
    frames4 = []
    cmp = spec["compare"]
    for r, (key, short) in enumerate((("serif-italic-christmas", "Serif italic, Christmas"), ("liturgy-easter", "Liturgy, Easter"))):
        yy = 150 + r * (fh + 170)
        for c, (suffix, label) in enumerate((("", "A the sign"), ("-B", "B the emblem"))):
            wf = spec["wireframes"][key + suffix]
            x = 40 + c * (fw + 60)
            frames4.append(dict(x=x, y=yy + 50, w=fw, h=fh, vw=W, vh=H, key=key + suffix))
            m = wf["mark"]
            w4.rect(x + m["x"] * scale - 4, yy + 50 + m["y"] * scale - 4, m["size"] * scale + 8, m["size"] * scale + 8, None, stroke="#E0201B", sw=2, name="callout")
            sheet_text(w4, x, yy + 36, "%s: %s, %d px" % (short, label, m["size"]), 28, "#1D1A15")
            hits = wf.get("overlaps")
            cap = "covers nothing: the mark's rectangle meets no other drawn element" if not hits else "covers: " + ", ".join(hits)
            sheet_text(w4, x, yy + 50 + fh + 34, cap, 26, "#5A564E", 500, file="InstrumentSans-500.ttf")
    ty = 150 + 2 * (fh + 170) + 10
    lines = [
        "Share of the sheet: A %s%%, B %s%%." % (cmp["area_a_pct"], cmp["area_b_pct"]),
        "B's top right rectangle lands on a drawn element in %d of 17 families (free only in d2 and d11), its bottom right one inside the body area in %d of 17; A meets nothing in the four wireframes." % (cmp["top_hits"], cmp["bottom_hits"]),
        "Where B fits (d11's empty corner, drawn here) it is a poster, not a data card; A takes 72 px of one existing line, which the kicker already gives up when it is too long.",
        "Recommended: A. B stays as the size for a gallery tile or a title-only card with no table (the church packs' posters carry no data).",
    ]
    for i, t in enumerate(lines):
        sheet_text(w4, 40, ty + 40 + i * 44, t, 28, "#1D1A15", 500, file="InstrumentSans-500.ttf")
    sheets["compare"] = dict(width=SW4, height=SH4, items=w4.el, frames=frames4)
    return sheets


# ------------------------------------------------------------------ writers
def glyph_svg(name, g, grid):
    title = g["title"].replace("&", "&amp;")
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 %d %d" width="%d" height="%d" role="img"><title>%s</title>%s</svg>\n' % (grid["view"], grid["view"], grid["view"], grid["view"], title, g["body"])


def write_marks(spec):
    os.makedirs(os.path.join(ROOT, "marks"), exist_ok=True)
    lines = ["// Season marks for the Rune icon painter: view 24, one colour (currentColor). Paste where the other icons live; not compiled from here."]
    for name, g in spec["glyphs"].items():
        open(os.path.join(ROOT, g["file"]), "w", encoding="utf-8", newline="\n").write(glyph_svg(name, g, spec["grid"]))
        const = "SEASON_" + name.upper().replace("-", "_")
        lines.append('pub static %s: Icon = Icon { name: "season-%s", view: 24.0, body: r#"%s"# };' % (const, name, g["body"]))
    open(os.path.join(ROOT, "marks", "icons.rs.txt"), "w", encoding="utf-8", newline="\n").write("\n".join(lines) + "\n")
    return len(spec["glyphs"])


def bodies(spec):
    out = {k: v["body"] for k, v in spec["glyphs"].items()}
    # weight tests: "name@weight"
    for k in list(out):
        for wgt in spec["grid"]["weights_tested"]:
            out["%s@%s" % (k, wgt)] = out[k].replace('stroke-width="2"', 'stroke-width="%s"' % wgt, 1)
    return out


def rebuild(spec_wire):
    w = wire.Wire()
    w.el = spec_wire["el"]
    w.defs = spec_wire["defs"]
    return w


def write_wireframes(spec):
    os.makedirs(os.path.join(ROOT, "wireframes"), exist_ok=True)
    b = bodies(spec)
    for key, wf in spec["wireframes"].items():
        svg = wire.serialize(rebuild(wf), b, spec["grid"]["view"])
        open(os.path.join(ROOT, "wireframes", key + ".svg"), "w", encoding="utf-8", newline="\n").write(svg)
    return len(spec["wireframes"])


def render(svg_path, png_path, extra=""):
    cmd = 'bunx @resvg/resvg-js-cli --no-system-font --font-dir "%s" %s "%s" "%s"' % (fonts.FONT_DIR, extra, svg_path, png_path)
    r = subprocess.run(cmd, shell=True, capture_output=True, timeout=300, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr)


def sheet_svg(spec, name, b):
    sh = spec["sheets"][name]
    plain = [e for e in sh["items"] if e.get("name") != "callout"]
    call = [e for e in sh["items"] if e.get("name") == "callout"]

    def inner(items):
        w = wire.Wire()
        w.el = items
        s = wire.serialize(w, b, spec["grid"]["view"], sh["width"], sh["height"])
        return s[s.index(">") + 1: s.rindex("</svg>")]

    head = '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">' % (sh["width"], sh["height"], sh["width"], sh["height"])
    parts = [head, inner(plain)]
    for fr in sh.get("frames", []):
        wf = spec["wireframes"][fr["key"]]
        body = wire.serialize(rebuild(wf), b, spec["grid"]["view"])
        body = body.replace('<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080">', '<svg x="%s" y="%s" width="%s" height="%s" viewBox="0 0 %s %s">' % (fr["x"], fr["y"], fr["w"], fr["h"], fr["vw"], fr["vh"]), 1)
        body = body.replace('id="wash"', 'id="wash-%s"' % fr["key"]).replace("url(#wash)", "url(#wash-%s)" % fr["key"]).replace('id="ground"', 'id="ground-%s"' % fr["key"]).replace("url(#ground)", "url(#ground-%s)" % fr["key"])
        parts.append(body)
    parts.append(inner(call))
    parts.append("</svg>")
    return "\n".join(parts)


def write_sheets(spec):
    b = bodies(spec)
    outs = {"contact": "contact-sheet", "sizes": "marks-sizes", "chips": "colourways-sheet", "compare": "designs-compare"}
    for key, fn in outs.items():
        svg = sheet_svg(spec, key, b)
        p = os.path.join(ROOT, "build", fn + ".svg")
        open(p, "w", encoding="utf-8").write(svg)
        render(p, os.path.join(ROOT, fn + ".png"))
    return len(outs)


def write_colourways(spec):
    os.makedirs(os.path.join(ROOT, "colourways"), exist_ok=True)
    for n in range(1, 18):
        f = spec["families"][str(n)]
        lines = []
        for x in f["new"]:
            lines.append(json.dumps(dict(id=x["id"], season=x["season"], palette=x["palette"]), ensure_ascii=False, separators=(",", ":")))
        open(os.path.join(ROOT, "colourways", "d%d.json" % n), "w", encoding="utf-8", newline="\n").write("[" + ",\n".join(lines) + "]\n")
    return 17


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "all"
    if cmd in ("spec", "all"):
        spec = make_spec()
        print("spec written:", len(spec["glyphs"]), "glyphs,", sum(len(f["new"]) for f in spec["families"].values()), "new colourways,", len(spec["wireframes"]), "wireframes")
    spec = json.load(open(SPEC, encoding="utf-8"))
    if cmd in ("marks", "all"):
        print("marks written:", write_marks(spec))
    if cmd in ("wire", "all"):
        print("wireframes written:", write_wireframes(spec))
    if cmd in ("sheets", "all"):
        print("sheets rendered:", write_sheets(spec))
    if cmd in ("docs", "all"):
        print("colourway files:", write_colourways(spec))
