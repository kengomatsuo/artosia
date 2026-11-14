"""Fill templates-default.tpl.md from spec.json: every table with a number in it is generated, never typed.

ARTOSIA_FONTS=<dir> python -I docs.py      writes docs/plans/artosia/templates-default.md
"""
import io, json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import engine
from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = os.path.dirname(HERE)
spec = json.load(open(os.path.join(ROOT, "spec.json"), encoding="utf-8"))
C = spec["candidates"]
ORDER = ["a", "b", "c", "d"]
ROLES = [("title", "title"), ("kicker", "kicker"), ("event", "row (schedule event)"), ("event_hl", "highlighted row"), ("name", "name (birthday)"),
         ("fig", "big figure: date numeral"), ("total", "big figure: total"), ("number", "big figure: account number"), ("verse", "verse"), ("caption", "caption")]


def px(n):
    return f"{n:g}"


def family(T, role):
    r = T["type"][role]
    f = T["fonts"][r["font"]]
    return f"{f['family']} {f['weight']}{' italic' if f['italic'] else ''}"


def type_table(c):
    T = C[c]
    rows = ["| Role | Face | px on 1080 | share of height | tracking | case |", "|---|---|---|---|---|---|"]
    for key, label in ROLES:
        if key not in T["type"]:
            continue
        r = T["type"][key]
        rows.append(f"| {label} (`{key}`) | {family(T, key)} | {px(r['size'])} | {r['size'] / 10.8:.1f}% | {r.get('ls', 0) * 100:+.0f}% em | {'caps' if r.get('caps') else 'as typed'} |")
    return "\n".join(rows)


def palette_table(c):
    T = C[c]
    keys = list(T["colourways"][0]["palette"].keys())
    head = "| Role | " + " | ".join(f"{cw['id']} ({cw['name']})" for cw in T["colourways"]) + " |"
    rows = [head, "|---|" + "---|" * len(T["colourways"])]
    for k in keys:
        rows.append(f"| `{k}` | " + " | ".join(f"`{cw['palette'][k]}`" for cw in T["colourways"]) + " |")
    return "\n".join(rows)


def contrast_table(c):
    rows = ["| Pair (text on ground) | gate | " + " | ".join(cw["id"] for cw in C[c]["colourways"]) + " |", "|---|---|" + "---|" * len(C[c]["colourways"])]
    pairs = C[c]["pairs"]
    for i, (fg, bg, kind) in enumerate(pairs):
        cells = []
        for cw in C[c]["colourways"]:
            r = spec["contrast"][c][cw["id"]][i]
            cells.append(f"{r['ratio']:.2f}:1{' (7+)' if r['hall'] else ''}")
        rows.append(f"| `{fg}` on `{bg}` | {kind} {'4.5' if kind == 'text' else '3'} | " + " | ".join(cells) + " |")
    return "\n".join(rows)


def grid_table(c):
    g = C[c]["grid"]
    return "| " + " | ".join(f"`{k}` {px(v)}" for k, v in g.items()) + " |"


def fit_table(c):
    names = {"s01": "schedule, 8 rows, 4 days, 1 special", "s02": "birthdays, 30 names, one month, 7 highlighted", "s02x": "birthdays, two months, page 1/2 (15 names)",
             "s03": "offerings, 6 funds, 12 digits, total", "s04": "account with QR", "s04n": "account without QR", "s05": "verse, 60 words", "s06": "notice with picture", "s06n": "notice without picture"}
    rows = ["| Kind | What the layout settled at (measured with the real faces) |", "|---|---|"]
    for v, label in names.items():
        f = spec["fit"][c][v]
        txt = ", ".join(f"{k} {px(x) if isinstance(x, (int, float)) else x}" for k, x in f.items())
        rows.append(f"| {label} | {txt} |")
    return "\n".join(rows)


def stress_table():
    labels = {"event_77": "one 77-character event in the 8-row week", "names_30_long": "30 names with three of 30 to 32 characters (one month)", "name_44": "one 44-character name on the 30-name page",
              "rupiah_14": "a 14-digit fund amount", "two_months_30": "30 long names across two months"}
    rows = ["| Case | " + " | ".join(f"{k.upper()} {C[k]['name']}" for k in ORDER) + " |", "|---|" + "---|" * 4]
    for key, label in labels.items():
        cells = []
        for c in ORDER:
            r = spec["stress"][c][key]
            if r["ok"]:
                extra = f" at {px(r['size'])} px" if "size" in r else ""
                cells.append("holds" + extra + (f", total {px(r['total_size'])} px" if "total_size" in r else ""))
            else:
                cells.append("continues on a second slide")
        rows.append(f"| {label} | " + " | ".join(cells) + " |")
    return "\n".join(rows)


def latin_kb(path):
    opts = subset.Options()
    opts.layout_features = ["*"]
    opts.name_IDs = ["*"]
    opts.glyph_names = False
    opts.hinting = False
    font = TTFont(path)
    s = subset.Subsetter(opts)
    s.populate(unicodes=list(range(0x20, 0x7F)) + list(range(0xA0, 0x100)) + [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026])
    s.subset(font)
    b = io.BytesIO()
    font.save(b)
    return len(b.getvalue()) / 1024


def fonts_table():
    bundled = {"IBMPlexSansCondensed-500.ttf", "IBMPlexSansCondensed-600.ttf", "IBMPlexSansCondensed-700.ttf"}
    rows = ["| Candidate | Cuts to add (static TTF, digits baked where noted) | Already bundled | Full size | Latin subset |", "|---|---|---|---|---|"]
    for c in ORDER:
        T = C[c]
        files = sorted({f["file"] for f in T["fonts"].values()})
        add = [f for f in files if f not in bundled]
        have = [f for f in files if f in bundled]
        full = sum(os.path.getsize(os.path.join(engine.static_dir(), f)) for f in add) / 1024
        lat = sum(latin_kb(os.path.join(engine.static_dir(), f)) for f in add)
        rows.append(f"| {c.upper()} {T['name']} | {', '.join(add)} | {', '.join(have) or 'none'} | {full:.0f} KB | {lat:.0f} KB |")
    return "\n".join(rows)


def compare_table():
    def g(c, v, k):
        return spec["fit"][c][v][k]

    worst = {c: min(r["ratio"] for cws in spec["contrast"][c].values() for r in cws if r["kind"] == "text" and r["fg"] in ("ink", "band_ink", "block_ink", "pill_ink")) for c in ORDER}
    rows = [("Schedule row size, 8 rows (px)", lambda c: px(g(c, "s01", "size"))),
            ("Birthday name size, 30 names (px)", lambda c: px(g(c, "s02", "size"))),
            ("Birthday name size, page 1/2 two months (px)", lambda c: px(g(c, "s02x", "size"))),
            ("Widest name that fits three columns (px)", lambda c: px(round(g(c, "s02", "name_w")))),
            ("Offering row size (px)", lambda c: px(g(c, "s03", "size"))),
            ("Total figure (px), 15 characters", lambda c: px(g(c, "s03", "total_size") if "total_size" in spec["fit"][c]["s03"] else C[c]["type"]["total"]["size"])),
            ("Account number with QR (px)", lambda c: px(g(c, "s04", "number_size"))),
            ("Verse size, 60 words (px) and lines", lambda c: f"{px(g(c, 's05', 'size'))} and {g(c, 's05', 'lines')}"),
            ("Lowest ink on ground in any colourway", lambda c: f"{worst[c]:.2f}:1"),
            ("Accent hues", lambda c: {"a": "one orange-red, plus the wash", "b": "one red", "c": "one indigo", "d": "one yellow, inside black rows only"}[c]),
            ("Families", lambda c: {"a": "1 (Source Serif 4)", "b": "1 (Hanken Grotesk)", "c": "1 (Figtree)", "d": "2 (Plex Mono, Plex Sans Condensed)"}[c])]
    out = ["| | " + " | ".join(f"{c.upper()} {C[c]['name']}" for c in ORDER) + " |", "|---|" + "---|" * 4]
    for label, fn in rows:
        out.append(f"| {label} | " + " | ".join(fn(c) for c in ORDER) + " |")
    return "\n".join(out)


def mock_values(c):
    T = C[c]
    P = T["colourways"][0]["palette"]
    g = T["grid"]
    rows = ["| Mock value | Mock (1920x1080) | Built page, measured |", "|---|---|---|"]
    for key, label in ROLES:
        if key in T["type"]:
            r = T["type"][key]
            rows.append(f"| {label}: size and face | {px(r['size'])} px, {family(T, key)} | |")
    rows.append(f"| left and right margins | {px(g['mx'])} px and {px(1920 - g['x2'])} px | |")
    rows.append(f"| title baseline | {px(g['title_base'])} px | |")
    rows.append(f"| body area | {px(g['body_top'])} to {px(g['body_bottom'])} px | |")
    rows.append(f"| ground, ink, mute, accent (light) | {P['paper']}, {P['ink']}, {P['mute']}, {P.get('accent', P.get('pill', P.get('band_accent')))} | |")
    rows.append("| surfaces | ground, one highlight shape, one wash or band | |")
    return "\n".join(rows)


FILL = {"type": type_table, "palette": palette_table, "contrast": contrast_table, "grid": grid_table, "fit": fit_table, "mock": mock_values}


def run_check(*args):
    import subprocess
    r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "check.py"), *args], capture_output=True, text=True, env=os.environ.copy())
    return r.stdout.strip()


def main():
    tpl = open(os.path.join(HERE, "templates-default.tpl.md"), encoding="utf-8").read()

    def sub(m):
        name, arg = m.group(1), m.group(2)
        if name == "stress":
            return stress_table()
        if name == "fonts":
            return fonts_table()
        if name == "compare":
            return compare_table()
        if name == "check":
            return run_check()
        if name == "selftest":
            return run_check("--selftest")
        return FILL[name](arg)

    out = re.sub(r"\{\{(\w+)(?::(\w))?\}\}", sub, tpl)
    open(os.path.join(os.path.dirname(ROOT), "templates-default.md"), "w", encoding="utf-8").write(out)
    print("written", os.path.join(os.path.dirname(ROOT), "templates-default.md"), len(out.splitlines()), "lines")


if __name__ == "__main__":
    main()
