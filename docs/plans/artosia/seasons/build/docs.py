"""Fills docs/plans/artosia/seasons-design.md from seasons-design.tpl.md and spec.json.

python -I docs.py       writes the page; every table and snippet between <!-- gen:name --> and <!-- /gen:name --> is generated, and {{key}} is a count.
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import glyphs  # noqa: E402
import palettes  # noqa: E402
import spec_data  # noqa: E402

ROOT = os.path.dirname(HERE)
OUT = os.path.join(os.path.dirname(ROOT), "seasons-design.md")
spec = json.load(open(os.path.join(ROOT, "spec.json"), encoding="utf-8"))
F = spec["families"]
SEASON_NAMES = {"advent": "Advent", "christmas": "Christmas", "lent": "Lent", "easter": "Easter", "pentecost": "Pentecost"}

READ48 = {
    "advent": "reads: a candle with its flame",
    "advent-1": "reads: four candles, one flame; the count holds at 48 px",
    "advent-2": "reads: two flames",
    "advent-3": "reads: three flames",
    "advent-4": "reads: four flames",
    "christmas": "reads: a star at once",
    "epiphany": "reads: a book under a star",
    "lent": "reads: a cross; the hollow stays open at 48 px",
    "palm": "reads: a branch; at 48 px it is a leafy stroke, it needs its colour next to words",
    "good-friday": "reads: a cross standing in a ring of thorns",
    "easter": "reads: a sunrise over a hill with a stone; the stone is the weakest part at 48 px",
    "ascension": "reads: a cloud with a light above it",
    "pentecost": "reads: a bird with a flame above it; the flame is a speck at 48 px, the bird carries it",
    "reformation": "reads: an open book with lines of text",
    "harvest": "reads: an ear of wheat",
    "new-year": "reads: a single spark",
}
DRAWS = {
    "advent": "one candle: a solid body 6.6 wide and a teardrop flame",
    "advent-1": "four candles on a shallow tray, the first flame lit",
    "advent-2": "the same row, two flames",
    "advent-3": "the same row, three flames",
    "advent-4": "the same row, four flames",
    "christmas": "an eight point star, the vertical points longest, solid",
    "epiphany": "an open book in outline with a solid four point light above it",
    "lent": "a hollow Latin cross, beams 4 units wide",
    "palm": "one branch: a curved rachis and four pairs of leaflets",
    "good-friday": "a cross standing in a crown of thorns: a ring with five thorns on its far edge",
    "easter": "a hill with an open door, the stone rolled aside, a half sun and five rays above",
    "ascension": "a cloud in outline with a tall solid light rising out of it",
    "pentecost": "a dove in outline (body and one raised wing) with a solid flame above its head",
    "reformation": "an open Bible in outline with two lines of text on each page",
    "harvest": "one ear: a stem, three pairs of grains and a tip grain, all solid",
    "new-year": "a single solid four point spark",
}


def table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "|".join("---" for _ in head) + "|"]
    for r in rows:
        out.append("| " + " | ".join(str(c) for c in r) + " |")
    return "\n".join(out)


def gen_marks():
    rows = []
    for k in glyphs.ORDER:
        g = spec["glyphs"][k]
        rows.append(["`%s`" % g["file"].split("/")[-1], g["season"] or "by hand", DRAWS[k], READ48[k]])
    return table(["file in `marks/`", "season code", "what it draws", "at 48 px"], rows)


def gen_paint():
    rows = []
    for s in palettes.SEASONS:
        lk, dk = palettes.light_kit(s), palettes.dark_kit(s)
        users_l = [n for n in range(1, 18) if palettes.FAMILIES[n]["mode"][s] == "L" and any(x["season"] == s for x in F[str(n)]["new"])]
        rows.append([SEASON_NAMES[s], palettes.PAINT[s]["name"], "`%s` `%s` `%s`" % (lk["g"], lk["a"], lk["gt"]), "`%s` `%s` `%s`" % (dk["g"], dk["a"], dk["t"]), "light: " + ", ".join("d%d" % n for n in users_l)])
    return table(["season", "colours", "light: ground, accent, tint", "dark: ground, accent, ink", "new colourways drawn light"], rows)


def gen_season_mark():
    rows = []
    for code, mark in spec["season_mark"].items():
        rows.append([code, "`%s`" % ("advent-1` to `advent-4` by the week, `advent" if mark == "advent-N" else mark) if mark else "none"])
    for code in spec["by_hand_only"]:
        rows.append([code + " (not a season of the engine)", "`%s` by hand only" % code])
    return table(["`Season::code()`", "mark"], rows)


def gen_placement():
    rows = []
    for n in range(1, 18):
        f = F[str(n)]
        pl = f["placement"]
        if pl["spot"] == "none":
            rows.append(["d%d %s" % (n, f["short"]), "none", "-", "-", pl["rule"] + " " + pl["why"]])
            continue
        size = "%d" % pl["size"] if pl["spot"] != "ring" else "50% of the ring (88, 75 or 60)"
        role = pl["role"] if pl["spot"] != "ring" else "the ring's colour (pink, yellow or teal)"
        mp = f["mark_pairs"]
        low = min(m["lowest_ratio"] for m in mp) if mp else 0
        rows.append(["d%d %s" % (n, f["short"]), pl["spot"], size, ("%s on `%s`, lowest %.2f:1 over every colourway" % (("the ring's colour (pink, yellow or teal)" if pl["spot"] == "ring" else "`%s`" % role), pl["ground"], low)), pl["rule"] + " " + pl["why"] + (" Amends: " + pl["never"] if pl.get("exception") else "")])
    return table(["family", "spot", "px on the 1920 x 1080 sheet", "role and ground", "rule and reason"], rows)


def gen_compare():
    c = spec["compare"]
    rows = [
        ["Size", "48 px (%s%% of the sheet), 56 px in d4 and d11, 60 to 88 px inside d3's ring" % c["area_a_pct"], "160 px (%s%% of the sheet)" % c["area_b_pct"]],
        ["Meets a drawn element", "none in the four wireframes (tightest clearance in `check.py` output below)", "top right lands on a drawn element in %d of 17 families, bottom right inside the body area in %d of 17 (`spec.json` compare.rows)" % (c["top_hits"], c["bottom_hits"])],
        ["Church practice", "GKI Bromo's candle icon (yt-4Nzs8kJy2YU.jpg), the ash cross alone (cvs-ash-wednesday-ministry-graphics.jpg), a flame in place of a letter (yt-SaBMHGmaESI.jpg)", "the church packs' corner motifs (notes.md rows 1, 2, 5, 21), all on title-only graphics that carry no table"],
        ["Data kept clean", "yes: it takes 72 px of the kicker line, which already gives way when a church name is long", "no: it needs the corner the date range, the tag chip or the last rows already use"],
        ["Cost to build", "one `Item::Mark` on the Sheet and one token block per family (spot, size, role)", "the same primitive plus a reflow of the body in every layout file"],
        ["Families' written Never lists", "one clause amended (d6); d7 to d9 keep theirs by having no mark", "B is an icon too, at 3.3 times the size, in the four families whose lists ban icons (d6 to d9)"],
        ["Verdict", "recommended", "kept only as the 160 px size for a gallery tile or a title-only card"],
    ]
    return table(["", "A: the sign beside the kicker", "B: the corner emblem"], rows)


def gen_counts():
    new = sum(len(f["new"]) for f in F.values())
    retag = sum(1 for f in F.values() for x in f["new"] if x["retag"])
    rows = []
    for n in range(1, 18):
        f = F[str(n)]
        rows.append(["d%d %s" % (n, f["short"]), ", ".join(f["have"]) or "-", ", ".join(x["season"] for x in f["new"]), ", ".join("%s to %s" % (x["retag"], x["season"]) for x in f["new"] if x["retag"]) or "-"])
    return table(["family", "already tagged", "gaps filled below", "tag an existing colourway instead"], rows)


def gen_retag():
    rows = []
    for n in range(1, 18):
        f = F[str(n)]
        for x in f["new"]:
            if x["retag"]:
                rows.append(["d%d.json" % n, "`{\"id\":\"%s\",` becomes `{\"id\":\"%s\",\"season\":\"%s\",`" % (x["retag"], x["retag"], x["season"]), "%.3f" % x["nearest_de"], "`%s` below stays as the alternative if a dedicated look is wanted" % x["id"]])
    return table(["file", "edit", "swatch distance (OKLab)", "note"], rows)


def pair_label(p):
    return "`%s` on `%s`" % (p[0], p[1])


def mark_line(pl):
    if pl["spot"] == "none":
        return "none, the colourway alone"
    if pl["spot"] == "ring":
        return "inside the first neon ring, half its diameter (88, 75 or 60 px), in the ring's colour"
    where = {"kicker": "after the kicker", "sticker": "in the round sticker", "panel": "in the corner of the navy area", "cross": "in place of the small cross", "corner": "in the top right corner"}[pl["spot"]]
    return "%d px, `%s`, %s" % (pl["size"], pl["role"], where)


def gen_colourways(n):
    f = F[str(n)]
    lines = []
    new = f["new"]
    lines.append("**Mark:** %s. **Tagged already:** %s. **Gaps:** %s." % (mark_line(f["placement"]), ", ".join("%s `%s`" % (x["season"], x["id"]) for x in f["existing"] if x["season"]) or "none", ", ".join(x["season"] for x in new)))
    lines.append("")
    lines.append("```json")
    for x in new:
        lines.append(json.dumps(dict(id=x["id"], season=x["season"], palette=x["palette"]), ensure_ascii=False, separators=(",", ":")) + ",")
    lines.append("```")
    lines.append("")
    head = ["pair (text 4.5, large and mark 3)", "gate"] + ["%s `%s`" % (x["season"], x["id"]) for x in new]
    rows = []
    for i, p in enumerate(f["pairs"]):
        rows.append([pair_label(p), p[2] + " " + ("4.5" if p[2] == "text" else "3")] + ["%.2f" % x["ratios"][i] for x in new])
    lines.append(table(head, rows))
    notes = ["`%s` is within swatch distance %.3f of the existing `%s`: tag it instead (see the retag table)." % (x["id"], x["nearest_de"], x["retag"]) for x in new if x["retag"]]
    if notes:
        lines.append("")
        lines += notes
    return "\n".join(lines)


def gen_mocktable():
    rows = []
    for key, wf in spec["wireframes"].items():
        if key.endswith("-B"):
            continue
        m = wf["mark"]
        n = wf["family"]
        pl = F[str(n)]["placement"]
        role = pl["role"] if n != 3 else "ring colour"
        col = m["color"]
        rows.append([key, "%s px" % m["size"], "x %s, y %s (top left)" % (m["x"], m["y"]), role, "`%s`" % col, "`%s`" % wf["ground"], "%.2f:1" % __import__("colour").contrast(col, wf["ground"]), "_", "_", "_"])
    return table(["wireframe (mock)", "size", "position", "role", "colour", "surface", "ratio", "built size", "built position", "built colour"], rows)


def gen_check():
    r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "check.py")], capture_output=True, text=True, timeout=600)
    return "```\n" + r.stdout.strip() + "\n```"


def gen_selftest():
    r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "check.py"), "--selftest"], capture_output=True, text=True, timeout=600)
    return "```\n" + r.stdout.strip() + "\n```"


def gen_retag_count():
    return str(sum(1 for f in F.values() for x in f["new"] if x["retag"]))


def main():
    tpl = open(os.path.join(HERE, "seasons-design.tpl.md"), encoding="utf-8").read()
    new = sum(len(f["new"]) for f in F.values())
    retag = sum(1 for f in F.values() for x in f["new"] if x["retag"])
    counts = {"n_new": str(new), "n_retag": str(retag), "n_made": str(new - retag), "n_templates": str(105 + new - retag), "n_placed": str(sum(1 for f in F.values() if f["placement"]["spot"] != "none")), "n_pairs": str(sum(len(f["pairs"]) * len(f["new"]) for f in F.values())), "advent_week_rule": spec["advent_week_rule"]}
    blocks = {"marks": gen_marks, "paint": gen_paint, "season_mark": gen_season_mark, "placement": gen_placement, "compare": gen_compare, "counts": gen_counts, "retag": gen_retag, "mocktable": gen_mocktable, "check": gen_check, "selftest": gen_selftest}
    for n in range(1, 18):
        blocks["cw%d" % n] = (lambda n=n: gen_colourways(n))
    for name, fn in blocks.items():
        pat = re.compile(r"<!-- gen:%s -->.*?<!-- /gen:%s -->" % (name, name), re.S)
        if not pat.search(tpl):
            continue
        body = fn()
        tpl = pat.sub(lambda m: "<!-- gen:%s -->\n%s\n<!-- /gen:%s -->" % (name, body, name), tpl)
    for k, v in counts.items():
        tpl = tpl.replace("{{%s}}" % k, v)
    open(OUT, "w", encoding="utf-8", newline="\n").write(tpl)
    print("written", OUT, len(tpl.splitlines()), "lines")


if __name__ == "__main__":
    main()
