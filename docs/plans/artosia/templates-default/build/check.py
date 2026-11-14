"""Check script: every number in a wireframe must be in spec.json, and every wireframe must keep the layout rules.

ARTOSIA_FONTS=<dir> python -I check.py            prints the numbers an SVG has that spec.json lacks, one line per rule, then the counts.
ARTOSIA_FONTS=<dir> python -I check.py --selftest  plants a bad number, a bad colour and a collision in a copy and shows the check flagging each.
Exit 1 on any failure.
"""
import glob, json, os, re, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import engine
from engine import Face, contrast

ROOT = os.path.dirname(HERE)
spec = json.load(open(os.path.join(ROOT, "spec.json"), encoding="utf-8"))
fails = []

NUM = re.compile(r"-?\d+(?:\.\d+)?")


def collect(node, out):
    if isinstance(node, bool):
        return
    if isinstance(node, (int, float)):
        out.add(round(float(node), 2))
    elif isinstance(node, dict):
        for v in node.values():
            collect(v, out)
    elif isinstance(node, list):
        for v in node:
            collect(v, out)


GROUP = re.compile(r"\d+(?:[.:/\-]\d+)*")


def leaf_digits(node, out):
    """The digit groups every leaf of the data holds: a group is a run such as 07.00, 12.500.000 or 831-2745-906, kept as its digits."""
    if isinstance(node, bool):
        return
    if isinstance(node, (int, float)):
        out.add(str(int(node)).lstrip("0") or "0")
    elif isinstance(node, str):
        for g in GROUP.findall(node):
            out.add(re.sub(r"\D", "", g).lstrip("0") or "0")
    elif isinstance(node, dict):
        for v in node.values():
            leaf_digits(v, out)
    elif isinstance(node, list):
        for v in node:
            leaf_digits(v, out)


allowed = set()
collect(spec, allowed)
data_digits = set()
leaf_digits(spec["data"], data_digits)
leaf_digits(spec["derived"], data_digits)
SVG_NUMBERS = re.compile(r'\s(?:x|y|x1|x2|y1|y2|cx|cy|r|rx|width|height|font-size|font-weight|letter-spacing|stroke-width|opacity|stop-opacity|offset)="(-?[\d.]+)"')
HEX = re.compile(r"#[0-9A-Fa-f]{6}\b")
TEXT = re.compile(r"<text [^>]*>(.*?)</text>")


def scan(svg, palette):
    """Numbers in the SVG's attributes and numerals in its text that spec.json lacks, and colours outside the palette."""
    body = re.sub(r"<style>.*?</style>", "", svg, flags=re.S)
    lacking = sorted({m for m in SVG_NUMBERS.findall(body) if round(float(m), 2) not in allowed}, key=float)
    text_lacking = []
    for t in TEXT.findall(body):
        for g in GROUP.findall(t):
            d = re.sub(r"\D", "", g).lstrip("0") or "0"
            if d not in data_digits:
                text_lacking.append(g)
    colours = sorted({c.upper() for c in HEX.findall(body)} - {v.upper() for v in palette.values()})
    return lacking, sorted(set(text_lacking)), colours


DESC = set("gjpqy,;()")


def boxes(items):
    out = []
    for it in items:
        if it["k"] != "text":
            continue
        face = Face.of(engine.font_file(it["family"], it["weight"], it["italic"]))
        top = it["y"] - face.cap * it["size"] - 0.02 * it["size"]
        bot = it["y"] + (0.2 if any(ch in DESC for ch in it["t"]) else 0.02) * it["size"]
        out.append((it["x"], top, it["x"] + it["w"], bot, it))
    return out


def collisions(items):
    bs = boxes(items)
    hits = []
    for i in range(len(bs)):
        for j in range(i + 1, len(bs)):
            a, b = bs[i], bs[j]
            if a[0] < b[2] - 1 and b[0] < a[2] - 1 and a[1] < b[3] - 1 and b[1] < a[3] - 1:
                hits.append((a[4]["id"], b[4]["id"]))
    return hits


def covered_by_shape(items):
    """Every text whose role ends in _hl sits on a filled shape drawn before it (shape plus weight, never colour alone)."""
    bad = []
    rects = [(i, it) for i, it in enumerate(items) if it["k"] == "rect" and it["fill"] and it["fill"] not in ("paper",) and it["w"] < 1900]
    for i, it in enumerate(items):
        if it["k"] == "text" and it["role"] in ("name_hl", "event_hl"):
            cx, cy = it["x"] + it["w"] / 2, it["y"] - it["size"] * 0.3
            if not any(j < i and r["x"] <= cx <= r["x"] + r["w"] and r["y"] <= cy <= r["y"] + r["h"] for j, r in rects):
                bad.append(it["id"])
    return bad


def report(name, bad, detail=""):
    print(f"{'ok  ' if not bad else 'FAIL'} {name}{(': ' + str(bad)[:300]) if bad else ''}{detail}")
    if bad:
        fails.append(name)


def main():
    total_nums = 0
    n_svg = 0
    for c, per in spec["resolved"].items():
        T = spec["candidates"][c]
        files = sorted(glob.glob(os.path.join(ROOT, f"{c}-*.svg")) + glob.glob(os.path.join(ROOT, "other-kinds", f"{c}-*.svg")))
        for path in files:
            svg = open(path, encoding="utf-8").read()
            lack, tlack, cols = scan(svg, T["colourways"][0]["palette"])
            n_svg += 1
            total_nums += len(SVG_NUMBERS.findall(svg))
            if lack or tlack or cols:
                report(f"numbers {os.path.relpath(path, ROOT)}", {"attr": lack, "text": tlack, "colours": cols})
        for cw in T["colourways"]:
            for path in glob.glob(os.path.join(ROOT, "colourways", f"{c}-{cw['id']}-*.svg")):
                svg = open(path, encoding="utf-8").read()
                lack, tlack, cols = scan(svg, cw["palette"])
                n_svg += 1
                if lack or tlack or cols:
                    report(f"numbers {os.path.relpath(path, ROOT)}", {"attr": lack, "text": tlack, "colours": cols})
    print(f"{n_svg} SVG scanned, {total_nums} numeric attributes in the root and other-kinds SVGs, all looked up in spec.json")

    over, tiny, small_body, margin, hl = [], [], [], [], []
    BODY = ("name", "name_hl", "event", "event_hl", "fund", "verse", "n_body", "note")
    for c, per in spec["resolved"].items():
        for vid, items in per.items():
            for a, b in collisions(items):
                over.append(f"{c}/{vid}: {a} x {b}")
            for it in items:
                if it["k"] == "text":
                    if it["size"] < 32:
                        tiny.append(f"{c}/{vid}: {it['id']} {it['size']}")
                    if it["role"] in BODY and it["size"] < 40:
                        small_body.append(f"{c}/{vid}: {it['id']} {it['size']}")
                    if it["x"] < 72 or it["x"] + it["w"] > 1848 or it["y"] > 1070:
                        margin.append(f"{c}/{vid}: {it['id']} x={it['x']} to {round(it['x'] + it['w'])} y={it['y']}")
            for t in covered_by_shape(items):
                hl.append(f"{c}/{vid}: {t}")
    report("no two texts collide", over)
    report("no text under 32 px", tiny)
    report("body rows and names at 40 px or more", small_body)
    report("every text inside the safe area (72 px sides, 1070 px bottom)", margin)
    report("every highlighted row sits on a shape", hl)
    pairs = [(c, cw, r) for c, cws in spec["contrast"].items() for cw, rows in cws.items() for r in rows if not r["pass"]]
    report("every listed text-on-ground pair passes its gate in every colourway", pairs, f" ({sum(len(rows) for cws in spec['contrast'].values() for rows in cws.values())} pairs)")
    nocw = [c for c, cws in spec["contrast"].items() if len(cws) != 3]
    report("three colourways per candidate", nocw)
    words = [(c, k) for c in spec["resolved"] for k in ("s02",) if spec["fit"][c][k]["size"] < 40]
    report("30 names hold in three columns at 40 px or more", words)
    if fails:
        print(f"{len(fails)} rule(s) failed")
        sys.exit(1)
    print("all rules pass")


def selftest():
    path = os.path.join(ROOT, "a-schedule.svg")
    svg = open(path, encoding="utf-8").read()
    bad = svg.replace('font-size="52"', 'font-size="53.7"', 1).replace("#FBF8F2", "#FBF8F3", 1).replace(">07.00<", ">07.45<", 1)
    lack, tlack, cols = scan(bad, spec["candidates"]["a"]["colourways"][0]["palette"])
    print("planted: font-size 53.7, colour #FBF8F3, time 07.45")
    print("flagged attr:", lack, "| text:", tlack, "| colours:", cols)
    items = json.loads(json.dumps(spec["resolved"]["a"]["s01"]))
    t = [i for i in items if i["k"] == "text"]
    t[3]["x"], t[3]["y"] = t[4]["x"], t[4]["y"]
    print("planted collision flagged:", collisions(items)[:2])
    ok = bool(lack) and bool(tlack) and bool(cols) and bool(collisions(items))
    print("selftest", "passes (every plant was flagged)" if ok else "FAILS")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    selftest() if "--selftest" in sys.argv else main()
