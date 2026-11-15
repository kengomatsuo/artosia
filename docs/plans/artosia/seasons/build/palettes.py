"""Seasonal colourways for the 17 warta families (SEASONMARK1).

The colour of each season comes from one paint box (kit) in OKLCH, light and dark; each family maps its own palette keys to
kit roles, so a colourway has the family's own keys and the family's own structure. A contrast solver then moves lightness
(never hue) until every pair the family lists passes, and the result is written to spec.json by build.py.

Run through build.py, never alone.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from colour import contrast, delta_e, hex_of, to_oklch  # noqa: E402
DIRS = os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "..", "crates", "artosia-slides", "assets", "warta", "directions"))

SEASONS = ["advent", "christmas", "lent", "easter", "pentecost"]
NEED = {"text": 4.5, "large": 3.0, "mark": 3.0}
MARGIN = 0.05  # the Rust test compares f32; stay clear of the line

# Season paint box. hg ground hue, ha accent hue, h2 second (metal or flame) hue, per light/dark mode.
# L = light ground, D = dark ground. c* are chromas. Every value here is data and is copied to spec.json.
PAINT = {
    "advent": {
        "name": "violet and candle gold",
        "L": dict(hg=310, cg=0.018, ha=306, ca=0.14, ct=0.040, h2=88, ho=45, co=0.15, hw=310, cw=0.06),
        "D": dict(hg=309, cg=0.060, ha=305, ca=0.11, h2=80, ca2=0.11, hw=305),
    },
    "christmas": {
        "name": "evergreen, holly red and gold",
        "L": dict(hg=125, cg=0.022, ha=156, ca=0.09, ct=0.045, h2=25, ho=25, co=0.15, hw=150, cw=0.07),
        "D": dict(hg=168, cg=0.045, ha=158, ca=0.10, h2=86, ca2=0.11, hw=160),
    },
    "lent": {
        "name": "ash and mulberry",
        "L": dict(hg=20, cg=0.007, ha=348, ca=0.11, ct=0.035, h2=20, ho=30, co=0.10, hw=350, cw=0.045),
        "D": dict(hg=340, cg=0.035, ha=345, ca=0.09, h2=8, ca2=0.075, hw=345),
    },
    "easter": {
        "name": "ivory and bronze gold",
        "L": dict(hg=95, cg=0.040, ha=76, ca=0.10, ct=0.075, h2=90, ho=62, co=0.15, hw=92, cw=0.10),
        "D": dict(hg=92, cg=0.050, ha=80, ca=0.10, h2=92, ca2=0.12, hw=88),
    },
    "pentecost": {
        "name": "red and flame",
        "L": dict(hg=30, cg=0.020, ha=27, ca=0.17, ct=0.040, h2=45, ho=40, co=0.19, hw=40, cw=0.09),
        "D": dict(hg=28, cg=0.060, ha=27, ca=0.14, h2=55, ca2=0.11, hw=30),
    },
}


def light_kit(s):
    p = PAINT[s]["L"]
    return {
        "g": hex_of(0.935, p["cg"], p["hg"]),
        "g2": hex_of(0.975, p["cg"] * 0.5, p["hg"]),
        "g3": hex_of(0.20, 0.035, p["hg"] if s != "easter" else 85),
        "gt": hex_of(0.90, p["ct"], p["ha"]),
        "t": hex_of(0.215, 0.028, p["hg"] if s != "lent" else 340),
        "t2": hex_of(0.44, 0.04, p["hg"] if s != "lent" else 340),
        "r": hex_of(0.86, p["cg"] * 1.3 + 0.004, p["hg"]),
        "a": hex_of(0.42, p["ca"], p["ha"]),
        "ad": hex_of(0.31, p["ca"] * 0.85, p["ha"]),
        "al": hex_of(0.80, 0.08, p["ha"]),
        "a2": hex_of(0.80, 0.14, p["h2"]),
        "a2d": hex_of(0.50, 0.12, p["h2"]),
        "o": hex_of(0.76, p["co"], p["ho"]),
        "w": hex_of(0.87, p["cw"], p["hw"]),
        "bi": hex_of(0.94, 0.02, p["hg"]),
        "bm": hex_of(0.76, 0.05, p["hg"]),
        "white": "#FFFFFF",
    }


def dark_kit(s):
    p = PAINT[s]["D"]
    cgd = p["cg"]
    return {
        "g": hex_of(0.245, cgd, p["hg"]),
        "g2": hex_of(0.30, cgd * 1.1, p["hg"]),
        "g3": hex_of(0.17, cgd * 0.75, p["hg"]),
        "gt": hex_of(0.32, cgd * 1.5, p["ha"]),
        "t": hex_of(0.955, 0.02, 75),
        "t2": hex_of(0.82, 0.045, p["hg"]),
        "r": hex_of(0.38, cgd * 1.2, p["hg"]),
        "a": hex_of(0.84, p["ca2"], p["h2"]),
        "ad": hex_of(0.70, p["ca2"] + 0.02, p["h2"]),
        "al": hex_of(0.80, 0.10, p["ha"]),
        "a2": hex_of(0.84, p["ca2"], p["h2"]),
        "a2d": hex_of(0.70, p["ca2"] + 0.02, p["h2"]),
        "w": hex_of(0.36, 0.10, p["ha"]),
        "white": "#FFFFFF",
    }


# Fixed colours a family keeps in every colourway (read from its own first colourway; see families below).
D3_NEON = {  # four neons on night, per season (L .75 to .92)
    "advent": ["#FF5CC0", "#FFD84A", "#6FE8FF", "#B596FF"],
    "christmas": ["#FF5C73", "#FFD65C", "#4DF2A6", "#8CD9FF"],
    "lent": ["#FF8FB8", "#F2D27A", "#8FD9D0", "#C7A6FF"],
    "easter": ["#FF7AA8", "#FFE45C", "#7CF0B0", "#B79CFF"],
    "pentecost": ["#FF4D4D", "#FFC83D", "#FF8A3C", "#FF7AA8"],
}
# D13 Block: signal band colour and the deep block, per season (OKLCH L, C, H).
D13_BLOCK = {
    "advent": ((0.74, 0.15, 303), (0.33, 0.16, 300)),
    "christmas": ((0.78, 0.19, 150), (0.34, 0.13, 22)),
    "lent": ((0.78, 0.09, 350), (0.32, 0.10, 345)),
    "easter": ((0.88, 0.15, 95), (0.40, 0.10, 70)),
    "pentecost": ((0.66, 0.23, 28), (0.31, 0.12, 263)),
}
# D16 signal foot per season: (L, C, H), and the ink on it (t = dark, white = white).
D16_SIGNAL = {
    "advent": ((0.47, 0.19, 297), "white"),
    "christmas": ((0.83, 0.14, 85), "g"),
    "lent": ((0.40, 0.11, 348), "white"),
    "easter": ((0.86, 0.15, 92), "t"),
    "pentecost": ((0.47, 0.19, 28), "white"),
}
D16_FUNDS_LIGHT = ["#C2410C", "#0F766E", "#1D4ED8", "#A21CAF", "#4D7C0F", "#B45309"]
D16_FUNDS_DARK = ["#FDBA74", "#5EEAD4", "#93C5FD", "#F0ABFC", "#BEF264", "#FCD34D"]

# Family role maps. A value is a kit role, ("hex", "#...") for a colour the family keeps, or a callable name handled in build_palette.
FAMILIES = {
    1: dict(mode={"advent": "L", "christmas": "D", "lent": "L", "easter": "L", "pentecost": "L"}, roles={"paper": "g", "card": "g2", "ink": "t", "mute": "t2", "accent": "a", "tint": "gt", "sage": "a2d"}),
    2: dict(mode={s: "L" for s in SEASONS}, roles={"ground": "g", "pill": "g3", "white": "g2", "yellow": ("hex", "#FFD23C"), "orange": "o", "burnt": "a", "ink": "g3", "mute": "t2", "on_pill": ("hex", "#FFFFFF")}),
    3: dict(mode={s: "D" for s in SEASONS}, roles={"night1": "g", "night2": "g3", "ink": "t", "mute": "t2", "pink": ("neon", 0), "yellow": ("neon", 1), "teal": ("neon", 2), "violet": ("neon", 3), "dark": "g3", "line": ("line", None), "white": ("hex", "#FFFFFF")}),
    4: dict(mode={s: "L" for s in SEASONS}, roles={"light": "g", "white": ("hex", "#FFFFFF"), "ink": "t", "blue": "a", "navy": "ad", "red": ("hex", "#C8161C"), "yellow": ("hex", "#FFC20E"), "hl": ("hex", "#FFEFB0"), "line": "r", "mute": "t2", "sky": "al", "on_navy": ("hex", "#FFFFFF")}),
    5: dict(mode={"advent": "L", "christmas": "D", "lent": "L", "easter": "L", "pentecost": "D"}, roles={"paper": "paper5", "white": ("hex", "#FFFFFF"), "ink": "t", "mute": "t2", "rule": "r", "accent": "a", "tint": "gt"}),
    6: dict(mode={"advent": "D", "christmas": "D", "lent": "L", "easter": "L", "pentecost": "L"}, roles={"paper": "g", "ink": "t", "mute": "t2", "rule": "r", "tint": "gt", "accent": "a", "wash": "w", "card": ("hex", "#FFFFFF")}),
    7: dict(mode={s: "L" for s in SEASONS}, roles={"paper": "g", "ink": "t", "mute": "t2", "rule": "r", "block": "g3", "block_ink": "bi", "block_mute": "bm", "accent": "a", "card": ("hex", "#FFFFFF")}),
    8: dict(mode={s: "L" for s in SEASONS}, roles={"paper": "g", "ink": "t", "mute": "t2", "rule": "r", "pill": "a", "pill_ink": ("hex", "#FFFFFF"), "card": ("hex", "#FFFFFF")}),
    9: dict(mode={s: "L" for s in SEASONS}, roles={"paper": "g", "ink": "t", "mute": "t2", "rule": "r", "band": "g3", "band_ink": "g", "band_mute": "bm", "band_accent": "a2", "card": ("hex", "#FFFFFF")}),
    10: dict(mode={s: "L" for s in SEASONS}, roles={"ground": "g", "panel": ("hex", "#FFFFFF"), "ink": "t", "mute": "t2", "brand": "a", "brand_dk": "ad", "on_brand": ("hex", "#FFFFFF"), "sky": "al", "tint": "gt", "line": "r"}),
    11: dict(mode={"advent": "D", "christmas": "D", "lent": "D", "easter": "L", "pentecost": "D"}, roles={"ground": ("vest", 0.36), "ground2": ("vest", 0.26), "gold": ("hex", "#E6C77C"), "cream": ("cream", None), "ink": ("vest", 0.17), "mute": ("vmute", None)}),
    12: dict(mode={s: "D" for s in SEASONS}, roles={"scrim": "g3", "plate": ("plate", None), "ink": ("ink12", None), "mute": ("mute12", None), "accent": ("accent12", None), "on_accent": ("hex", "#FFFFFF"), "white": ("hex", "#FFFFFF"), "dusk_a": ("dusk", 0.33), "dusk_b": ("dusk_b", None), "accent_tint": ("tint12", None)}),
    13: dict(mode={s: "D" for s in SEASONS}, roles={"signal": ("d13", 0), "deep": "g3", "field2": ("d13", 1), "white": ("hex", "#FFFFFF"), "mute": ("mute13", None)}),
    14: dict(mode={s: "D" for s in SEASONS}, roles={"g1": ("g1", None), "g2": ("g2m", None), "orb_a": ("orb_a", None), "orb_b": ("orb_b", None), "ink": ("ink14", None), "mute": ("mute14", None), "accent": ("acc14", None), "glass": ("hex", "#FFFFFF"), "pill": ("ink14", None), "on_pill": ("g1", None), "glass_g2": ("glass14", None)}),
    15: dict(mode={"advent": "D", "christmas": "L", "lent": "L", "easter": "L", "pentecost": "D"}, roles={"ground": "g", "ink": "t", "mute": "t2", "rule": "r15", "accent": "a", "inv_bg": "t", "inv_fg": "g", "card": ("hex", "#FFFFFF")}),
    16: dict(mode={"advent": "L", "christmas": "D", "lent": "L", "easter": "L", "pentecost": "L"}, roles={"ground": "g", "cell": "cell16", "ink": "t", "mute": "t2", "signal": ("sig16", 0), "on_signal": ("sig16", 1), "fund1": ("fund", 0), "fund2": ("fund", 1), "fund3": ("fund", 2), "fund4": ("fund", 3), "fund5": ("fund", 4), "fund6": ("fund", 5), "on_fund": "onfund16"}),
    17: dict(mode={"advent": "D", "christmas": "D", "lent": "L", "easter": "L", "pentecost": "D"}, roles={"ground": "g", "card": "g2", "ink": "t", "mute": "t2", "accent": "a", "on_accent": "g", "plate": "plate17", "on_plate": "g", "line": "r"}),
}
# The dark kit's first light role is the text colour; some families need other derived colours. They are computed here from the kit.


def kit_for(season, mode):
    return light_kit(season) if mode == "L" else dark_kit(season)


def resolve(fam, season, key, role, kit):
    p = PAINT[season]
    mode = FAMILIES[fam]["mode"][season]
    pm = p[mode]
    if isinstance(role, str):
        if role in kit:
            return kit[role]
        if role == "rule15" or role == "r15":
            return hex_of(0.72 if mode == "L" else 0.42, 0.03, pm["hg"])
        if role == "cell16":
            return "#FFFFFF" if mode == "L" else hex_of(0.30, pm["cg"] * 1.1, pm["hg"])
        if role == "onfund16":
            return "#FFFFFF" if mode == "L" else kit["g"]
        if role == "paper5":
            return kit["g2"] if mode == "L" else kit["g"]
        if role == "plate17":
            return kit["g3"] if mode == "L" else kit["t"]
        if role == "g1":
            return hex_of(0.20, 0.05, pm["hg"])
        raise KeyError(role)
    kind, arg = role
    if kind == "hex":
        return arg
    if kind == "neon":
        return D3_NEON[season][arg]
    if kind == "line":
        return hex_of(0.40, 0.13, pm["ha"])
    if kind == "vest":
        c = {"advent": 0.10, "christmas": 0.075, "lent": 0.040, "easter": 0.065}.get(season, 0.10)
        hue = {"advent": 305, "christmas": 160, "lent": 335, "easter": 85}.get(season, 27)
        return hex_of(arg, c if arg > 0.2 else c * 0.7, hue)
    if kind == "cream":
        return hex_of(0.96, 0.03, 85)
    if kind == "vmute":
        return hex_of(0.92, 0.03, pm["hg"])
    if kind == "plate":
        return hex_of(0.98, 0.008, pm["hg"])
    if kind == "ink12":
        return hex_of(0.20, 0.04, pm["hg"] if season != "easter" else 60)
    if kind == "mute12":
        return hex_of(0.44, 0.06, pm["hg"] if season != "easter" else 60)
    if kind == "accent12":
        return hex_of(0.42, 0.12 if season != "lent" else 0.10, pm["ha"] if season != "easter" else 55)
    if kind == "dusk":
        return hex_of(arg, 0.10, pm["ha"])
    if kind == "dusk_b":
        return hex_of(0.75, 0.11, pm["h2"] if season != "lent" else 10)
    if kind == "tint12":
        return hex_of(0.91, 0.04, pm["ha"] if season != "easter" else 70)
    if kind == "d13":
        return hex_of(*D13_BLOCK[season][arg])
    if kind == "mute13":
        return hex_of(0.83, 0.02, pm["hg"])
    if kind == "g2m":
        return hex_of(0.40, 0.12, pm["ha"])
    if kind == "orb_a":
        return hex_of(0.85, 0.11, pm["h2"])
    if kind == "orb_b":
        return hex_of(0.68, 0.18, pm["ha"] if season != "christmas" else 145)
    if kind == "ink14":
        return hex_of(0.97, 0.02, pm["hg"])
    if kind == "mute14":
        return hex_of(0.83, 0.05, pm["hg"])
    if kind == "acc14":
        return hex_of(0.90, 0.08, pm["h2"])
    if kind == "glass14":
        return hex_of(0.46, 0.11, pm["ha"])
    if kind == "g1":
        return hex_of(0.20, 0.05, pm["hg"])
    if kind == "sig16":
        (lch, on) = D16_SIGNAL[season]
        if arg == 0:
            return hex_of(*lch)
        return {"white": "#FFFFFF", "t": kit["t"], "g": kit["g"]}[on]
    if kind == "fund":
        return (D16_FUNDS_LIGHT if mode == "L" else D16_FUNDS_DARK)[arg]
    raise KeyError(kind)


def load(fam):
    return json.load(open(os.path.join(DIRS, "d%d.json" % fam), encoding="utf-8"))


def d11_easter():
    """Liturgy at Easter is white and gold: an ivory vestment, deep gold, and the bands turn dark with ivory words."""
    return {"ground": hex_of(0.945, 0.04, 95), "ground2": hex_of(0.875, 0.07, 92), "gold": hex_of(0.46, 0.10, 80), "cream": hex_of(0.22, 0.04, 85), "ink": hex_of(0.96, 0.03, 92), "mute": hex_of(0.44, 0.05, 85)}


OVERRIDES = {(11, "easter"): d11_easter}


def build_palette(fam, season, d):
    keys = list(d["colourways"][0]["palette"].keys())
    if (fam, season) in OVERRIDES:
        pal = OVERRIDES[(fam, season)]()
        assert list(pal.keys()) == keys
        return pal
    roles = FAMILIES[fam]["roles"]
    assert set(roles) == set(keys), (fam, set(keys) ^ set(roles))
    mode = FAMILIES[fam]["mode"][season]
    kit = kit_for(season, mode)
    return {k: resolve(fam, season, k, roles[k], kit) for k in keys}


# Keys the solver must never move (fixed colours the family owns: white plates, QR plates, fund hues, brand yellows).
FIXED = {
    2: {"yellow", "on_pill"}, 4: {"white", "red", "yellow", "hl", "on_navy"}, 5: {"white"}, 6: {"card"}, 7: {"card"}, 8: {"pill_ink", "card"}, 9: {"card"},
    10: {"panel", "on_brand"}, 11: {"gold"}, 12: {"white", "on_accent"}, 13: {"white"}, 14: {"glass"}, 15: {"card"}, 16: {"fund1", "fund2", "fund3", "fund4", "fund5", "fund6"}, 3: {"white"},
}


def step_l(hexs, dl):
    L, C, H = to_oklch(hexs)
    return hex_of(min(max(L + dl, 0.02), 0.995), C, H)


def solve(fam, pal, pairs):
    """Move lightness of the foreground (or the background when the foreground is fixed) until every listed pair passes with a margin."""
    fixed = FIXED.get(fam, set())
    for _ in range(400):
        worst = None
        for fg, bg, kind in pairs:
            ratio = contrast(pal[fg], pal[bg])
            need = NEED[kind] + MARGIN
            if ratio < need and (worst is None or ratio - need < worst[0]):
                worst = (ratio - need, fg, bg)
        if worst is None:
            return True
        _, fg, bg = worst
        mover = fg if fg not in fixed else bg
        if mover in fixed:
            return False
        other = bg if mover == fg else fg
        lm, lo = to_oklch(pal[mover])[0], to_oklch(pal[other])[0]
        pal[mover] = step_l(pal[mover], 0.006 if lm >= lo else -0.006)
    return False


def pick_id(season, mode, taken):
    base = {"advent": ["adven"], "christmas": ["natal"], "lent": ["prapaskah"], "easter": ["paskah"], "pentecost": ["merah", "bara", "api"] if mode == "L" else ["bara", "api", "merah"]}[season]
    for b in base:
        if b not in taken:
            return b
    raise RuntimeError("no id")


def generate():
    out = {}
    for fam in range(1, 18):
        d = load(fam)
        have = {c["season"] for c in d["colourways"] if c.get("season")}
        taken = {c["id"] for c in d["colourways"]}
        pairs = d["pairs"]
        out[fam] = {"name": d["name"], "pairs": pairs, "keys": list(d["colourways"][0]["palette"].keys()), "have": sorted(have), "new": []}
        for s in SEASONS:
            if s in have:
                continue
            pal = build_palette(fam, s, d)
            ok = solve(fam, pal, pairs)
            cid = pick_id(s, FAMILIES[fam]["mode"][s], taken)
            taken.add(cid)
            ratios = [round(contrast(pal[a], pal[b]), 2) for a, b, _ in pairs]
            # nearest existing colourway by mean OKLab distance over the keys
            near = None
            sw = d["swatch"]
            for c in d["colourways"]:
                dist = max(delta_e(pal[k], c["palette"][k]) for k in sw)
                if near is None or dist < near[1]:
                    near = (c["id"], dist, c.get("season"))
            out[fam]["new"].append({"id": cid, "season": s, "mode": FAMILIES[fam]["mode"][s], "palette": pal, "ok": ok, "ratios": ratios, "nearest": near[0], "nearest_de": round(near[1], 3), "nearest_season": near[2]})
    return out


if __name__ == "__main__":
    res = generate()
    bad = 0
    for fam, r in res.items():
        for n in r["new"]:
            flag = "OK " if n["ok"] else "FAIL"
            bad += 0 if n["ok"] else 1
            print("D%-2d %-9s %-10s %s min %.2f nearest %s %.3f" % (fam, n["season"], n["id"], flag, min(n["ratios"]), n["nearest"], n["nearest_de"]))
    print("new colourways", sum(len(r["new"]) for r in res.values()), "failed", bad)
