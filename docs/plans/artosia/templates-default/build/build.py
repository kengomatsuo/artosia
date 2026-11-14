"""Driver: tokens + data -> display lists (spec.json "resolved") -> SVG -> PNG -> contact sheet.

ARTOSIA_FONTS=<dir made by make_fonts.py> python -I build.py [layout] [svg] [png] [sheet]     (no argument: all)
"""
import copy, glob, importlib, json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import engine
from engine import Sheet, Split, contrast, write_svg

ROOT = os.path.dirname(HERE)
TOK = os.path.join(ROOT, "tokens")
SPEC = os.path.join(ROOT, "spec.json")
CANDS = ["a", "b", "c", "d"]
# variant -> (layout function key, kwargs, file stem, folder)
VARIANTS = {
    "s01": ("s01", {}, "schedule", ""),
    "s02": ("s02", {"key": "s02"}, "birthdays", ""),
    "s02x": ("s02", {"key": "s02x", "part": "part", "caption": "caption"}, "birthdays-two-months-1of2", ""),
    "s03": ("s03", {}, "offerings", "other-kinds"),
    "s04": ("s04", {"qr_on": True}, "account", "other-kinds"),
    "s04n": ("s04", {"qr_on": False}, "account-noqr", "other-kinds"),
    "s05": ("s05", {}, "verse", "other-kinds"),
    "s06": ("s06", {"photo": True}, "notice", "other-kinds"),
    "s06n": ("s06", {"photo": False}, "notice-nopic", "other-kinds"),
}
NEED = {"text": 4.5, "large": 3.0, "mark": 3.0}


def load():
    spec = {"canvas": {"w": engine.W, "h": engine.H}, "candidates": {}}
    spec["data"] = json.load(open(os.path.join(TOK, "data.json"), encoding="utf-8"))
    for c in CANDS:
        p = os.path.join(TOK, c + ".json")
        if os.path.exists(p):
            spec["candidates"][c] = json.load(open(p, encoding="utf-8"))
    return spec


def mods():
    return {c: importlib.import_module(c) for c in CANDS if os.path.exists(os.path.join(HERE, c + ".py"))}


def run_one(mod, T, data, vid, cw="light"):
    fn, kw, _, _ = VARIANTS[vid]
    sh = Sheet(T, data, cw)
    kw = dict(kw)
    for field in ("caption", "part"):
        if kw.get(field) == field:
            kw[field] = data["s02x"][field]
    mod.LAYOUT[fn](sh, data, **kw)
    return sh


def contrast_table(T):
    out = {}
    for cw in T["colourways"]:
        P = cw["palette"]
        rows = []
        for fg, bg, kind in T["pairs"]:
            r = contrast(P[fg], P[bg])
            rows.append({"fg": fg, "bg": bg, "kind": kind, "ratio": round(r, 2), "need": NEED[kind], "pass": r >= NEED[kind], "hall": r >= 7.0})
        out[cw["id"]] = rows
    return out


def stress(spec, ms):
    """The same layouts on data worse than the artboards': what each candidate does with it."""
    res = {}
    st = spec["data"]["stress"]

    def cases():
        def long_events(d):
            d["s01"]["rows"][3]["event"] = st["event_77"]

        def long_names(d):
            for i, nm in st["names_long"].items():
                d["s02"]["names"][int(i)]["name"] = nm

        def name44(d):
            d["s02"]["names"][10]["name"] = st["name_44"]

        def rupiah14(d):
            d["s03"]["funds"][0]["rupiah"] = st["rupiah_14"]

        yield "event_77", "s01", long_events
        yield "names_30_long", "s02", long_names
        yield "name_44", "s02", name44
        yield "rupiah_14", "s03", rupiah14
        yield "two_months_30", "s02full", lambda d: None

    for c, mod in ms.items():
        T = spec["candidates"][c]
        res[c] = {}
        for name, vid, mut in cases():
            data = copy.deepcopy(spec["data"])
            mut(data)
            try:
                if vid == "s02full":
                    data["s02"] = data["s02full"]
                    sh = run_one(mod, T, data, "s02")
                else:
                    sh = run_one(mod, T, data, vid)
                res[c][name] = {"ok": True, **sh.fit}
            except Split as e:
                res[c][name] = {"ok": False, "why": str(e)}
    return res


def layout(spec):
    ms = mods()
    resolved, fits = {}, {}
    for c, mod in ms.items():
        T = spec["candidates"][c]
        resolved[c], fits[c] = {}, {}
        for vid in VARIANTS:
            sh = run_one(mod, T, spec["data"], vid)
            resolved[c][vid] = sh.items
            fits[c][vid] = sh.fit
    spec["resolved"], spec["fit"] = resolved, fits
    spec["derived"] = {"total_gifts": sum(f["rupiah"] for f in spec["data"]["s03"]["funds"]), "note": "figures a slide shows that no data row holds"}
    spec["contrast"] = {c: contrast_table(spec["candidates"][c]) for c in ms}
    spec["stress"] = stress(spec, ms)
    return spec


def save(spec):
    json.dump(spec, open(SPEC, "w", encoding="utf-8"), ensure_ascii=False, indent=1)


def out_path(c, vid, ext):
    _, _, stem, folder = VARIANTS[vid]
    d = os.path.join(ROOT, folder)
    os.makedirs(d, exist_ok=True)
    return os.path.join(d, f"{c}-{stem}.{ext}")


def cw_path(c, cw, vid, ext):
    d = os.path.join(ROOT, "colourways")
    os.makedirs(d, exist_ok=True)
    return os.path.join(d, f"{c}-{cw}-{VARIANTS[vid][2]}.{ext}")


def svgs(spec):
    for c, per in spec["resolved"].items():
        T = spec["candidates"][c]
        for vid, items in per.items():
            open(out_path(c, vid, "svg"), "w", encoding="utf-8").write(write_svg(T, "light", vid, items, f"{T['name']} {VARIANTS[vid][2]}"))
        for cw in T["colourways"]:
            for vid in ("s01", "s02"):
                open(cw_path(c, cw["id"], vid, "svg"), "w", encoding="utf-8").write(write_svg(T, cw["id"], vid, per[vid], f"{T['name']} {cw['name']} {VARIANTS[vid][2]}"))


def pngs(spec, folder_png):
    os.makedirs(folder_png, exist_ok=True)
    files = []
    jobs = []
    for c, per in spec["resolved"].items():
        for vid in per:
            jobs.append(out_path(c, vid, "svg"))
        for cw in spec["candidates"][c]["colourways"]:
            jobs += [cw_path(c, cw["id"], vid, "svg") for vid in ("s01", "s02")]
    for svg in jobs:
        png = os.path.join(folder_png, os.path.basename(svg)[:-4] + ".png")
        subprocess.run(["bun", "x", "@resvg/resvg-js-cli", "--no-system-font", "--font-dir", engine.static_dir(), "--fit-width", "1920", svg, png], check=True, capture_output=True)
        files.append(png)
    print(len(files), "PNG in", folder_png)


def main():
    args = sys.argv[1:] or ["layout", "svg", "png", "sheet"]
    spec = load()
    if "layout" in args:
        spec = layout(spec)
        save(spec)
    else:
        spec = json.load(open(SPEC, encoding="utf-8"))
    if "svg" in args:
        svgs(spec)
    if "png" in args:
        pngs(spec, os.environ.get("ARTOSIA_PNG", os.path.join(HERE, "_png")))
    if "sheet" in args:
        import sheets
        sheets.run(spec, os.environ.get("ARTOSIA_PNG", os.path.join(HERE, "_png")), ROOT)


if __name__ == "__main__":
    main()
