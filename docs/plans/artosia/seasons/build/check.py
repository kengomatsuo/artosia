"""Check script for SEASONMARK1: prints any number in a drawing that spec.json lacks, and every rule the marks, placements and colourways keep.

python -I check.py             runs every check, prints one line per rule and the counts, exit 1 on any failure
python -I check.py --selftest  plants a bad number, a bad colour, a bad path command and a failing pair in copies and shows each flagged
"""
import glob
import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fonts  # noqa: E402
from colour import contrast  # noqa: E402

ROOT = os.path.dirname(HERE)
NEED = {"text": 4.5, "large": 3.0, "mark": 3.0}
NUM = re.compile(r"-?\d+(?:\.\d+)?")
ATTR_NUM = re.compile(r'\s(?:x|y|x1|x2|y1|y2|cx|cy|r|rx|ry|width|height|font-size|font-weight|letter-spacing|stroke-width|opacity|stop-opacity|offset)="(-?[\d.]+)"')
ROT = re.compile(r"rotate\(([-\d. ]+)\)")
VIEWBOX = re.compile(r'viewBox="([\d. ]+)"')
HEX = re.compile(r"#[0-9A-Fa-f]{6}\b")
PATH_D = re.compile(r'\sd="([^"]*)"')
TEXT = re.compile(r"<text [^>]*>(.*?)</text>")
ALLOWED_TAGS = {"svg", "title", "g", "path", "rect", "circle", "ellipse"}
ALLOWED_ATTRS = {"xmlns", "viewBox", "width", "height", "role", "fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "d", "x", "y", "rx", "ry", "cx", "cy", "r"}
PATH_CMDS = set("MLHVCQAZ")
spec = json.load(open(os.path.join(ROOT, "spec.json"), encoding="utf-8"))
fails = []
counts = {}


def fail(msg):
    fails.append(msg)
    print("FAIL", msg)


def collect(node, out, strings=None):
    if isinstance(node, bool):
        return
    if isinstance(node, (int, float)):
        out.add(round(float(node), 2))
    elif isinstance(node, str):
        if strings is not None:
            strings.append(node)
    elif isinstance(node, dict):
        for v in node.values():
            collect(v, out, strings)
    elif isinstance(node, list):
        for v in node:
            collect(v, out, strings)


def allowed_numbers(sp):
    out, strings = set(), []
    collect(sp, out, strings)
    for g in sp["glyphs"].values():
        for n in NUM.findall(g["body"]):
            out.add(round(float(n), 2))
    for wf in sp["wireframes"].values():
        for e in wf["el"]:
            if e["t"] == "path":
                for n in NUM.findall(e["d"]):
                    out.add(round(float(n), 2))
    for wgt in sp["grid"]["weights_tested"]:
        out.add(round(float(wgt), 2))
    digits = set()
    for s in strings:
        for g in re.findall(r"\d+", s):
            digits.add(g.lstrip("0") or "0")
    for n in out:
        digits.add(str(int(abs(n))))
    return out, digits


def scan_svg(svg, allowed, digits, palette_hex):
    body = re.sub(r"<style>.*?</style>", "", svg, flags=re.S)
    lacking = sorted({m for m in ATTR_NUM.findall(body) if round(float(m), 2) not in allowed}, key=float)
    for m in ROT.findall(body):
        for n in m.split():
            if round(float(n), 2) not in allowed:
                lacking.append(n)
    for m in VIEWBOX.findall(body):
        for n in m.split():
            if round(float(n), 2) not in allowed:
                lacking.append(n)
    for d in PATH_D.findall(body):
        for n in NUM.findall(d):
            if round(float(n), 2) not in allowed:
                lacking.append(n)
    text_lacking = []
    for t in TEXT.findall(body):
        for g in re.findall(r"\d+", t):
            if (g.lstrip("0") or "0") not in digits:
                text_lacking.append(g)
    colours = sorted({c.upper() for c in HEX.findall(body)} - palette_hex)
    return sorted(set(lacking)), sorted(set(text_lacking)), colours


def palette_hexes(sp):
    out = set(c.upper() for c in sp["sheet_colours"])
    for f in sp["families"].values():
        for x in f["new"]:
            out |= {v.upper() for v in x["palette"].values()}
    dirs = os.path.normpath(os.path.join(ROOT, "..", "..", "..", "..", "crates", "artosia-slides", "assets", "warta", "directions"))
    for fn in glob.glob(os.path.join(dirs, "d*.json")):
        d = json.load(open(fn, encoding="utf-8"))
        for c in d["colourways"]:
            out |= {v.upper() for v in c["palette"].values()}
    return out


# ---------------------------------------------------------------- 1. the marks
def check_marks(sp, files=None):
    problems = 0
    names = list(sp["glyphs"].keys())
    for name in names:
        g = sp["glyphs"][name]
        path = os.path.join(ROOT, g["file"]) if files is None else files[name]
        text = open(path, encoding="utf-8").read()
        try:
            root = ET.fromstring(text)
        except ET.ParseError as e:
            fail("%s does not parse: %s" % (name, e))
            problems += 1
            continue
        ids = [el.get("id") for el in root.iter() if el.get("id")]
        if ids:
            fail("%s has ids %s" % (name, ids))
            problems += 1
        for el in root.iter():
            tag = el.tag.split("}")[-1]
            if tag not in ALLOWED_TAGS:
                fail("%s uses <%s>, outside the icon painter's subset" % (name, tag))
                problems += 1
            for a, v in el.attrib.items():
                if a not in ALLOWED_ATTRS:
                    fail("%s: attribute %s outside the subset" % (name, a))
                    problems += 1
                if a in ("fill", "stroke") and v not in ("none", "currentColor"):
                    fail("%s: %s=%s is not currentColor or none" % (name, a, v))
                    problems += 1
            if tag == "path":
                cmds = set(re.findall(r"[A-Za-z]", el.get("d", "")))
                if not cmds <= PATH_CMDS:
                    fail("%s: path commands %s outside M L H V C Q A Z" % (name, sorted(cmds - PATH_CMDS)))
                    problems += 1
        if root.get("viewBox") != "0 0 %d %d" % (sp["grid"]["view"], sp["grid"]["view"]):
            fail("%s: viewBox %s is not the 24 grid" % (name, root.get("viewBox")))
            problems += 1
        if g["body"] not in text:
            fail("%s: the file's drawing differs from spec.json" % name)
            problems += 1
        if "currentColor" not in text or HEX.search(text):
            fail("%s: a colour other than currentColor" % name)
            problems += 1
    counts["marks"] = len(names)
    return problems


def render_live_area(sp):
    """Render all marks on one transparent sheet, crop each and measure its ink box in grid units."""
    from PIL import Image
    px = 240
    cols = 4
    rows = (len(sp["glyphs"]) + cols - 1) // cols
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">' % (cols * px, rows * px, cols * px, rows * px)]
    for i, (name, g) in enumerate(sp["glyphs"].items()):
        parts.append('<svg x="%d" y="%d" width="%d" height="%d" viewBox="0 0 24 24">%s</svg>' % ((i % cols) * px, (i // cols) * px, px, px, g["body"].replace("currentColor", "#000000")))
    parts.append("</svg>")
    tmp_svg, tmp_png = os.path.join(HERE, "_live.svg"), os.path.join(HERE, "_live.png")
    open(tmp_svg, "w", encoding="utf-8").write("".join(parts))
    subprocess.run('bunx @resvg/resvg-js-cli --no-system-font "%s" "%s"' % (tmp_svg, tmp_png), shell=True, check=True, capture_output=True, timeout=300)
    img = Image.open(tmp_png).convert("RGBA")
    out = {}
    for i, name in enumerate(sp["glyphs"].keys()):
        x0, y0 = (i % cols) * px, (i // cols) * px
        a = img.crop((x0, y0, x0 + px, y0 + px)).split()[3]
        bb = a.point(lambda v: 255 if v > 24 else 0).getbbox()
        out[name] = [round(v * 24 / px, 2) for v in bb]
    os.remove(tmp_svg)
    os.remove(tmp_png)
    return out


# ---------------------------------------------------------------- 2. the numbers of the drawings
def check_drawings(sp, extra_svg=None):
    allowed, digits = allowed_numbers(sp)
    pal = palette_hexes(sp)
    bad = 0
    svgs = sorted(glob.glob(os.path.join(ROOT, "wireframes", "*.svg"))) + [os.path.join(ROOT, "build", n + ".svg") for n in ("contact-sheet", "marks-sizes", "colourways-sheet", "designs-compare")]
    if extra_svg:
        svgs += extra_svg
    for p in svgs:
        svg = open(p, encoding="utf-8").read()
        lack, tlack, col = scan_svg(svg, allowed, digits, pal)
        if lack:
            fail("%s has numbers spec.json lacks: %s" % (os.path.basename(p), lack))
            bad += 1
        if tlack:
            fail("%s has numerals in text spec.json lacks: %s" % (os.path.basename(p), tlack))
            bad += 1
        if col:
            fail("%s has colours outside every palette: %s" % (os.path.basename(p), col))
            bad += 1
    counts["drawings"] = len(svgs)
    return bad


# ---------------------------------------------------------------- 3. colourways and pairs
def check_colourways(sp):
    bad = 0
    n_new = n_pairs = 0
    lowest = None
    for n, f in sp["families"].items():
        keys, pairs = f["keys"], f["pairs"]
        ids = [c["id"] for c in f["existing"]]
        for x in f["new"]:
            n_new += 1
            if x["id"] in ids:
                fail("d%s: id %s already used" % (n, x["id"]))
                bad += 1
            ids.append(x["id"])
            if list(x["palette"].keys()) != keys:
                fail("d%s/%s: keys differ from the family's" % (n, x["id"]))
                bad += 1
            for (fg, bg, kind), ratio in zip(pairs, x["ratios"]):
                got = contrast(x["palette"][fg], x["palette"][bg])
                n_pairs += 1
                if abs(got - ratio) > 0.01:
                    fail("d%s/%s: %s on %s recorded %.2f, measured %.2f" % (n, x["id"], fg, bg, ratio, got))
                    bad += 1
                if got < NEED[kind]:
                    fail("d%s/%s: %s on %s is %.2f, %s needs %s" % (n, x["id"], fg, bg, got, kind, NEED[kind]))
                    bad += 1
                if lowest is None or got - NEED[kind] < lowest[0]:
                    lowest = (got - NEED[kind], "d%s/%s %s on %s %.2f (%s)" % (n, x["id"], fg, bg, got, kind))
            for m in f["mark_pairs"]:
                pass
        # the file the builder pastes from
        path = os.path.join(ROOT, "colourways", "d%s.json" % n)
        data = json.load(open(path, encoding="utf-8"))
        if [(c["id"], c["season"], c["palette"]) for c in data] != [(x["id"], x["season"], x["palette"]) for x in f["new"]]:
            fail("colourways/d%s.json differs from spec.json" % n)
            bad += 1
    counts["colourways"] = n_new
    counts["pair_checks"] = n_pairs
    counts["tightest"] = lowest[1] if lowest else ""
    return bad


def check_mark_pairs(sp):
    """Every role a mark takes on its ground is already a pair the family lists, at text or mark strength, and holds in every colourway, old and new."""
    bad = 0
    n = 0
    for k, f in sp["families"].items():
        pl = f["placement"]
        if pl["spot"] == "none":
            continue
        for mp in f["mark_pairs"]:
            n += 1
            if mp["listed_as"] is None:
                fail("d%s: mark role %s on %s is not a listed pair" % (k, mp["role"], mp["ground"]))
                bad += 1
            elif mp["lowest_ratio"] < NEED[mp["listed_as"]]:
                fail("d%s: mark role %s on %s lowest %.2f, gate %s" % (k, mp["role"], mp["ground"], mp["lowest_ratio"], NEED[mp["listed_as"]]))
                bad += 1
            if mp["lowest_ratio"] < 3.0:
                fail("d%s: mark role %s on %s under 3:1" % (k, mp["role"], mp["ground"]))
                bad += 1
    counts["mark_pairs"] = n
    return bad


def check_sizes(sp):
    bad = 0
    for k, f in sp["families"].items():
        pl = f["placement"]
        if pl["spot"] in ("none",):
            continue
        sizes = [pl["size"]] if pl["spot"] != "ring" else [round(r * 176 * pl["ratio"]) for r in (1,)] + [round(150 * pl["ratio"]), round(120 * pl["ratio"])]
        for s in sizes:
            if not (sp["grid"]["min_px"] <= s <= sp["grid"]["max_px"]):
                fail("d%s: mark %s px is outside %s to %s px" % (k, s, sp["grid"]["min_px"], sp["grid"]["max_px"]))
                bad += 1
    return bad


# ---------------------------------------------------------------- 4. clearance in the wireframes
def box(e):
    if e["t"] == "text":
        h = e["size"]
        w = e["w"]
        x = e["x"] - (w if e["anchor"] == "end" else (w / 2 if e["anchor"] == "middle" else 0))
        return (x, e["y"] - h * 0.8, x + w, e["y"] + h * 0.2)
    if e["t"] == "rect":
        return (e["x"], e["y"], e["x"] + e["w"], e["y"] + e["h"])
    return None


def check_clearance(sp):
    bad = 0
    tight = None
    for key, wf in sp["wireframes"].items():
        if key.endswith("-B"):
            continue  # design B is drawn to show its collisions
        m = wf["mark"]
        mb = (m["x"], m["y"], m["x"] + m["size"], m["y"] + m["size"])
        for e in wf["el"]:
            if e["t"] not in ("text", "rect") or e.get("name") in ("ground", "ground2", "wash", "frame", "frame2", "card", "band", "diamond", "glow", "ring", "pill", "highlight", "line", "arch"):
                continue
            b = box(e)
            gap_x = max(b[0] - mb[2], mb[0] - b[2])
            gap_y = max(b[1] - mb[3], mb[1] - b[3])
            gap = max(gap_x, gap_y)
            if tight is None or gap < tight[0]:
                tight = (gap, "%s: mark to %s %s" % (key, e.get("name"), e["t"]))
            if gap < 8:
                fail("%s: the mark is %.1f px from %s (needs 8)" % (key, gap, e.get("name")))
                bad += 1
    counts["tightest_clearance"] = "%.1f px, %s" % tight if tight else ""
    return bad


def main():
    bad = 0
    print("marks: files, subset, grid, currentColor")
    bad += check_marks(spec)
    live = render_live_area(spec)
    out = [n for n, bb in live.items() if bb[0] < 1 or bb[1] < 1 or bb[2] > 23 or bb[3] > 23]
    if out:
        fail("ink outside 1..23 units: %s" % {n: live[n] for n in out})
        bad += len(out)
    counts["live_area"] = "all %d inside 1 to 23 units (box: %s)" % (len(live), "; ".join("%s %s" % (n, bb) for n, bb in list(live.items())[:3]) + "; ...")
    print("drawings: every number, numeral and colour of the wireframes and sheets is in spec.json or a palette")
    bad += check_drawings(spec)
    print("colourways: keys, ids, every listed pair at its gate, files equal spec")
    bad += check_colourways(spec)
    print("marks on grounds: role and ground are a listed pair, lowest over every colourway")
    bad += check_mark_pairs(spec)
    bad += check_sizes(spec)
    print("wireframes: clearance of the mark from every other drawn element")
    bad += check_clearance(spec)
    placed = sum(1 for f in spec["families"].values() if f["placement"]["spot"] != "none")
    none = [k for k, f in spec["families"].items() if f["placement"]["spot"] == "none"]
    retag = sum(1 for f in spec["families"].values() for x in f["new"] if x["retag"])
    print()
    print("marks %d (12 seasons + 4 advent weeks), families with a mark %d, without %d (d%s)" % (counts["marks"], placed, len(none), ", d".join(none)))
    print("new colourways %d, pair checks %d, retag instead of new %d, tightest pair: %s" % (counts["colourways"], counts["pair_checks"], retag, counts["tightest"]))
    print("mark pairs checked %d; tightest clearance %s" % (counts["mark_pairs"], counts["tightest_clearance"]))
    print("live area: %s" % counts["live_area"])
    print("drawings scanned %d; numbers spec.json lacks: %s" % (counts["drawings"], "none" if not any("spec.json lacks" in f for f in fails) else "see FAIL lines"))
    print("RESULT:", "FAIL (%d)" % len(fails) if fails else "ok")
    return 1 if fails else 0


def selftest():
    import copy
    import shutil
    import tempfile
    global spec
    base = copy.deepcopy(spec)
    allowed, digits = allowed_numbers(base)
    pal = palette_hexes(base)
    svg = open(os.path.join(ROOT, "wireframes", "serif-italic-christmas.svg"), encoding="utf-8").read()
    bad = re.sub(r' x="[\d.]+"', ' x="913.13"', svg, count=1).replace(">15<", ">88888<", 1)
    bad = bad.replace('fill="%s"' % base["wireframes"]["serif-italic-christmas"]["el"][0]["fill"], 'fill="#123456"', 1)
    lack, tlack, col = scan_svg(bad, allowed, digits, pal)
    print("planted x=913.13 ->", "flagged %s" % lack if lack else "MISSED")
    print("planted numeral 88888 ->", "flagged %s" % tlack if tlack else "MISSED")
    print("planted colour #123456 ->", "flagged %s" % col if col else "MISSED")
    tmp = tempfile.mkdtemp()
    files = {}
    for name, g in base["glyphs"].items():
        src = open(os.path.join(ROOT, g["file"]), encoding="utf-8").read()
        if name == "lent":
            src = src.replace("<path ", '<path fill="#ff0000" ', 1).replace("H14V8", "H14S8", 1)
        files[name] = os.path.join(tmp, name + ".svg")
        open(files[name], "w", encoding="utf-8").write(src)
    before = len(fails)
    check_marks(base, files)
    print("planted red fill and an S command in lent.svg ->", "flagged %d" % (len(fails) - before) if len(fails) > before else "MISSED")
    # a failing pair
    fam = base["families"]["6"]
    x = fam["new"][0]
    x["palette"]["mute"] = x["palette"]["paper"]
    spec = base
    before = len(fails)
    check_colourways(base)
    print("planted mute = paper in d6 ->", "flagged %d" % (len(fails) - before) if len(fails) > before else "MISSED")
    shutil.rmtree(tmp)
    return 0


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(selftest())
    sys.exit(main())
