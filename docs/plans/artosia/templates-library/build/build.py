"""The library's one tool: python -I build.py <step> [args]

  layout FONT_DIR   resolve every wireframe scene into spec.json "generated" (geometry, contrast, fits)
  draw              write the SVG wireframes from spec.json alone
  check             print every number an SVG holds that spec.json lacks, failing contrast, glyph and ledger gaps
  render FONT_DIR WORK_DIR   Chrome headless: one PNG per SVG, then the contact sheet PNG
  doc               fill the generated tables of ../../templates-library.md from spec.json
"""
import glob, os, re, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import HERE, OUT, SPEC, Fonts, load, save_generated, numbers_in, NUM, pair_ratio, r2, rupiah, wrap
from families import LAYOUTS

KINDS = ("S02", "S03")
SLUG = {"S02": "birthdays", "S03": "offerings"}
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
MD = os.path.join(os.path.dirname(OUT), "templates-library.md")


def svg_name(fid, kind):
    return f"{fid}-{kind}-{SLUG[kind]}.svg"


# ------------------------------------------------------------------ layout

# ------------------------------------------------------------------ the four kinds that are not drawn

INDONESIAN_LONGER = 1.12  # an Indonesian verse runs about this much wider than the English fixture (UNSURE: no 60-word Indonesian sample is on disk)


def kinds_fit(spec, fonts, fid):
    """What the four undrawn kinds do at their worst case, measured with the family's own faces."""
    fam, D = spec["families"][fid], spec["data"]
    K = fam["kinds"]

    def face(role):
        key, size, ls, caps, _ = fam["type"][role]
        return fam["fonts"][key][3], size, ls, caps

    out = {}
    stem, size, ls, caps = face("event")
    k = K["s01"]
    by_height = int((k["bot"] - k["top"] - k["gaps"] * k["gap"]) / (k["rows_eq"] * k["ratio"]))
    use = min(size, by_height)
    events = [r[3] for r in D["schedule"]]
    longest = max(events, key=lambda e: fonts.width(stem, e, use, ls))
    out["s01"] = dict(rows=len(events), days=len({r[0] for r in D["schedule"]}), special=sum(r[4] for r in D["schedule"]), role_size=size, size_by_height=by_height, size=use,
                      longest_event=longest, longest_px=r2(fonts.width(stem, longest, use, ls)), event_w=k["event_w"], lines=len(wrap(fonts, stem, longest, use, k["event_w"], ls)))
    stem, size, ls, caps = face("number")
    num = D["account"]["number"]
    for name, width in (("with_qr", K["s04"]["w_qr"]), ("without_qr", K["s04"]["w"])):
        z = size
        while z > 1 and fonts.width(stem, num, z, ls) > width:
            z -= 1
        halves = [num[:len(num) // 2].rstrip(), num[len(num) // 2:].lstrip()]
        two = z
        if z < K["s04"]["floor"]:
            two = size
            while two > 1 and max(fonts.width(stem, h, two, ls) for h in halves) > width:
                two -= 1
        out.setdefault("s04", {})[name] = dict(one_line_size=z, size=z if z >= K["s04"]["floor"] else two, lines=1 if z >= K["s04"]["floor"] else 2, width=width)
    out["s04"]["digits"] = len(num.replace(" ", ""))
    stem, size, ls, caps = face("verse")
    words = D["verse60"]
    k = K["s05"]
    for name, width in (("with_picture", k["w_pic"]), ("without_picture", k["w"])):
        z, lines = size, 0
        while z >= k["floor"]:
            lines = len(wrap(fonts, stem, words, z, width / INDONESIAN_LONGER, ls))
            if lines * k["lh"] * z <= k["bot"] - k["top"]:
                break
            z -= 1
        out.setdefault("s05", {})[name] = dict(size=z, lines=lines, height=r2(lines * k["lh"] * z), room=k["bot"] - k["top"], width=width, fits=z >= k["floor"])
    out["s05"]["words"] = len(words.split())
    stem, size, ls, caps = face("n_title")
    title = D["notice"]["title"].upper() if caps else D["notice"]["title"]
    k = K["s06"]
    for name, width in (("with_picture", k["w_pic"]), ("without_picture", k["w"])):
        z = size
        while z > k["floor"] and len(wrap(fonts, stem, title, z, width, ls)) > 3:
            z -= 1
        out.setdefault("s06", {})[name] = dict(size=z, lines=len(wrap(fonts, stem, title, z, width, ls)), width=width, role_size=size)
    return out

def check_data(spec):
    d = spec["data"]
    names = d["birthdays"]["names"]
    assert len(names) == 30, len(names)
    assert sum(n[3] for n in names) == 7, "seven highlighted"
    assert len({n[1] for n in names}) == 2, "a page across two months"
    assert [(n[1], n[0]) for n in names] == sorted((n[1], n[0]) for n in names), "names in date order"
    assert len(d["offerings"]["funds"]) == 6
    assert max(len(str(f[1])) for f in d["offerings"]["funds"]) == 12, "twelve-digit amounts"


def layout(font_dir):
    spec = load()
    check_data(spec)
    fonts = Fonts(font_dir)
    gen = {"scenes": {}, "contrast": {}, "figures": {}, "kinds": {}}
    thr = spec["thresholds"]
    for fid, (s02, s03) in LAYOUTS.items():
        for kind, fn in zip(KINDS, (s02, s03)):
            sc = fn(spec, fonts)
            gen["scenes"][f"{fid}/{kind}"] = {"w": sc.W, "h": sc.H, "colourway": spec["families"][fid]["colourways"][0]["id"], "fits": sc.fits, "els": sc.els}
        fam = spec["families"][fid]
        gen["kinds"][fid] = kinds_fit(spec, fonts, fid)
        gen["contrast"][fid] = {}
        for cw in fam["colourways"]:
            rows = []
            for fg, bg, kind in fam["pairs"]:
                ratio = pair_ratio(cw["palette"], fam.get("layers", {}), fg, bg)
                rows.append([fg, bg, kind, round(ratio, 2), ratio >= thr[kind]])
            gen["contrast"][fid][cw["id"]] = rows
        gen["figures"][fid] = {k: ("tabular" if len({fonts.face(v[3])[1][fonts.face(v[3])[0][c]][0] for c in range(0x30, 0x3A)}) == 1 else "proportional") for k, v in fam["fonts"].items()}
    save_generated(gen)
    bad = [(f, c, r) for f, cws in gen["contrast"].items() for c, rows in cws.items() for r in rows if not r[4]]
    print(f"layout: {len(gen['scenes'])} scenes, {sum(len(r) for c in gen['contrast'].values() for r in c.values())} contrast pairs, {len(bad)} failing")
    for f, c, r in bad:
        print("  FAIL", f, c, r)
    for key, s in gen["scenes"].items():
        f = s["fits"]
        print(f"  {key:14s} elements={len(s['els']):4d} name_size={f.get('name_size', '-')!s:4s} shortened={f.get('names_shortened', '-')!s:3s} total_px={f.get('total_px', '-')} label_lines={f.get('label_lines', '-')}")


# ------------------------------------------------------------------ draw

def n(v):
    return str(r2(v))


def attrs(**kw):
    return "".join(f' {k.replace("_", "-")}="{v}"' for k, v in kw.items() if v is not None)


def draw_scene(spec, fid, kind, cw=0):
    fam = spec["families"][fid]
    pal = fam["colourways"][cw]["palette"]
    sc = spec["generated"]["scenes"][f"{fid}/{kind}"]
    col = lambda k: pal[k] if k else None
    defs, body = [], []
    for i, el in enumerate(sc["els"]):
        k = el["k"]
        op = n(el["op"]) if "op" in el else None
        if k == "r":
            tr = f"rotate({n(el['rot'])} {n(el['rcx'])} {n(el['rcy'])})" if el.get("rot") else None
            body.append(f"<rect{attrs(x=n(el['x']), y=n(el['y']), width=n(el['w']), height=n(el['h']), rx=n(el['rx']) if 'rx' in el else None, fill=col(el.get('c')) or 'none', stroke=col(el.get('st')), stroke_width=n(el['sw']) if 'sw' in el else None, stroke_dasharray=el.get('dash'), opacity=op, transform=tr)}/>")
        elif k == "p":
            body.append(f"<polygon{attrs(points=' '.join(n(a) + ',' + n(b) for a, b in el['pts']), fill=col(el['c']), opacity=op)}/>")
        elif k == "o":
            body.append(f"<circle{attrs(cx=n(el['cx']), cy=n(el['cy']), r=n(el['r']), fill=col(el.get('c')) or 'none', stroke=col(el.get('st')), stroke_width=n(el['sw']) if 'sw' in el else None, opacity=op)}/>")
        elif k == "l":
            body.append(f"<line{attrs(x1=n(el['x1']), y1=n(el['y1']), x2=n(el['x2']), y2=n(el['y2']), stroke=col(el['st']), stroke_width=n(el['sw']), stroke_dasharray=el.get('dash'), opacity=op)}/>")
        elif k == "d":
            d = " ".join(n(t) if isinstance(t, (int, float)) else t for t in el["d"])
            body.append(f"<path{attrs(d=d, fill='none', stroke=col(el['st']), stroke_width=n(el['sw']), opacity=op)}/>")
        elif k == "g":
            stops = "".join(f'<stop{attrs(offset=n(o), stop_color=pal[c], stop_opacity=n(a))}/>' for o, c, a in el["stops"])
            defs.append(f'<radialGradient id="g{i}" gradientUnits="userSpaceOnUse"{attrs(cx=n(el["cx"]), cy=n(el["cy"]), r=n(el["r"]))}>{stops}</radialGradient>')
            body.append(f'<circle{attrs(cx=n(el["cx"]), cy=n(el["cy"]), r=n(el["r"]), fill=f"url(#g{i})")}/>')
        elif k == "f":
            defs.append(f'<linearGradient id="f{i}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{pal[el["a"]]}"/><stop offset="1" stop-color="{pal[el["b"]]}"/></linearGradient>')
            body.append(f'<rect{attrs(x=n(el["x"]), y=n(el["y"]), width=n(el["w"]), height=n(el["h"]), fill=f"url(#f{i})")}/>')
            for j, (cx, cy, rx, ry, bop, blur, fx, fy, fw, fh) in enumerate(el["blobs"]):
                defs.append(f'<filter id="b{i}_{j}" filterUnits="userSpaceOnUse"{attrs(x=n(fx), y=n(fy), width=n(fw), height=n(fh))}><feGaussianBlur{attrs(stdDeviation=n(blur))}/></filter>')
                body.append(f'<ellipse{attrs(cx=n(cx), cy=n(cy), rx=n(rx), ry=n(ry), fill=pal[el["b"]], opacity=n(bop), filter=f"url(#b{i}_{j})")}/>')
        elif k == "t":
            fam_name, weight, italic, _ = fam["fonts"][el["f"]]
            text = el["s"].replace("&", "&amp;").replace("<", "&lt;")
            body.append(f"<text{attrs(x=n(el['x']), y=n(el['y']), font_family=fam_name, font_weight=weight, font_style='italic' if italic else None, font_size=n(el['z']), letter_spacing=(n(el['ls']) + 'em') if el.get('ls') else None, fill=col(el['c']), opacity=op, text_anchor=el['a'] if el['a'] != 'start' else None)}>{text}</text>")
    W, H = sc["w"], sc["h"]
    head = f'<svg xmlns="http://www.w3.org/2000/svg"{attrs(width=W, height=H, viewBox=f"0 0 {W} {H}")}>'
    desc = f"<title>{fam['name']} · {kind} {SLUG[kind]} · {fam['colourways'][cw]['id']}</title><desc>Generated by build/build.py draw from spec.json; do not edit by hand.</desc>"
    return head + desc + ("<defs>" + "".join(defs) + "</defs>" if defs else "") + "".join(body) + "</svg>\n"


def draw():
    spec = load()
    for fid in LAYOUTS:
        for kind in KINDS:
            open(os.path.join(OUT, svg_name(fid, kind)), "w", encoding="utf-8", newline="\n").write(draw_scene(spec, fid, kind))
    print(f"draw: {len(LAYOUTS) * len(KINDS)} SVGs in {OUT}")


# ------------------------------------------------------------------ check

SKIP = {"xmlns", "id", "fill", "stroke", "font-family", "text-anchor", "font-style", "stop-color", "filter", "stroke-dasharray"}


def svg_numbers(text):
    """(number, attribute, element) for every number in an attribute value of an SVG."""
    out = []
    for tag in re.finditer(r"<(\w+)([^>]*)>", text):
        for name, value in re.findall(r'([\w:-]+)="([^"]*)"', tag.group(2)):
            if name in SKIP and not value.startswith("#") or value.startswith(("url(", "#")) or name in SKIP:
                continue
            for m in NUM.findall(value):
                out.append((round(float(m), 2), name, tag.group(1)))
    return out


def missing_numbers(spec, text):
    have = set()
    numbers_in(spec, have)
    return [(v, a, e) for v, a, e in svg_numbers(text) if v not in have and -v not in have]


def check():
    spec = load()
    fails = 0
    have_all = set()
    numbers_in({k: v for k, v in spec.items() if k != "ledger"}, have_all)
    total = 0
    for fid in LAYOUTS:
        for kind in KINDS:
            path = os.path.join(OUT, svg_name(fid, kind))
            text = open(path, encoding="utf-8").read()
            nums = svg_numbers(text)
            miss = [(v, a, e) for v, a, e in nums if v not in have_all and -v not in have_all]
            total += len(nums)
            if miss:
                fails += 1
                print(f"MISSING in {os.path.basename(path)}: " + ", ".join(f"{v} ({e}@{a})" for v, a, e in sorted(set(miss))[:12]))
    print(f"numbers: {total} numbers in 16 SVG attributes, {fails} files hold a number spec.json lacks")
    # selftest: a number only the drawing knows must be caught
    sample = open(os.path.join(OUT, svg_name("diagonal", "S02")), encoding="utf-8").read()
    sample = re.sub(r'font-size="[\d.]+"', 'font-size="45.37"', sample, count=1)
    caught = any(v == 45.37 for v, _, _ in svg_numbers(sample) if v not in have_all)
    print(f"selftest: an invented font-size 45.37 is {'caught' if caught else 'NOT CAUGHT'}")
    fails += not caught
    # contrast
    thr = spec["thresholds"]
    rows = bad = 0
    for fid, cws in spec["generated"]["contrast"].items():
        low = 99
        for cid, pairs in cws.items():
            for fg, bg, kind, ratio, ok in pairs:
                rows += 1
                low = min(low, ratio / thr[kind])
                if ratio < thr[kind]:
                    bad += 1
                    print(f"CONTRAST {fid}/{cid} {fg} on {bg} {kind}: {ratio} < {thr[kind]}")
    print(f"contrast: {rows} pairs over 8 families x 4 colourways, {bad} below threshold")
    fails += bad
    # glyphs
    fonts = Fonts(os.environ.get("LIB_FONTS", ""))
    gaps = 0
    if os.environ.get("LIB_FONTS"):
        for key, sc in spec["generated"]["scenes"].items():
            fam = spec["families"][key.split("/")[0]]
            for el in sc["els"]:
                if el["k"] == "t":
                    miss = fonts.covers(fam["fonts"][el["f"]][3], el["s"])
                    if miss:
                        gaps += 1
                        print(f"GLYPHS {key} {el['f']} lacks {miss}")
        print(f"glyphs: every character of every drawn string is in its font: {gaps == 0}")
        fails += gaps
    # fits
    fit_fails = 0
    for key, sc in spec["generated"]["scenes"].items():
        f = sc["fits"]
        if any(x > 2 for x in f.get("label_lines", [])):
            fit_fails += 1
            print(f"FIT {key}: a label needs more than two lines")
        if f.get("name_size", 99) < 40:
            fit_fails += 1
            print(f"FIT {key}: name size {f['name_size']} is below the 40 px floor")
    fails += fit_fails
    print("fits: every fund label on at most two lines, every name at or above the 40 px floor" if not fit_fails else "fits: see above")
    # ledger
    ids = set(spec["families"])
    md = open(MD, encoding="utf-8").read() if os.path.exists(MD) else ""
    heads = {re.sub(r"[^a-z0-9]+", "-", h.lower()).strip("-") for h in re.findall(r"^#+\s+(.*)$", md, re.M)}
    rows_n = mapped = dropped = unmapped = 0
    for row in spec["ledger"]:
        rows_n += 1
        if row.get("dropped"):
            dropped += 1
            continue
        targets = row.get("maps_to", [])
        ok = bool(targets)
        for t in targets:
            if t.startswith("family:") and t[7:] not in ids:
                ok = False
            if t.startswith("section:") and t[8:] not in heads:
                ok = False
                print(f"LEDGER {row['id']}: no heading '{t[8:]}' in the spec")
            if t.startswith("file:") and not os.path.exists(os.path.join(OUT, t[5:])) and not os.path.exists(os.path.join(os.path.dirname(OUT), t[5:])):
                ok = False
                print(f"LEDGER {row['id']}: no file {t[5:]}")
        if ok:
            mapped += 1
        else:
            unmapped += 1
            print(f"LEDGER unmapped: {row['id']} {row['item']}")
    print(f"ledger: rows={rows_n} mapped={mapped} dropped={dropped} unmapped={unmapped}")
    fails += unmapped
    print("check:", "FAIL" if fails else "ok")
    sys.exit(1 if fails else 0)


# ------------------------------------------------------------------ render

def font_faces(spec, font_dir):
    seen, css = set(), []
    for fam in spec["families"].values():
        for family, weight, italic, stem in fam["fonts"].values():
            if (family, weight, italic) not in seen:
                seen.add((family, weight, italic))
                css.append(f"@font-face{{font-family:'{family}';font-weight:{weight};font-style:{'italic' if italic else 'normal'};src:url('file:///{font_dir.replace(chr(92), '/')}/{stem}.ttf')}}")
    return "\n".join(css)


def render(font_dir, work):
    from PIL import Image, ImageDraw, ImageFont
    spec = load()
    os.makedirs(work, exist_ok=True)
    css = font_faces(spec, os.path.abspath(font_dir))
    pngs = {}
    for fid in LAYOUTS:
        for kind in KINDS:
            svg = open(os.path.join(OUT, svg_name(fid, kind)), encoding="utf-8").read()
            page = os.path.join(work, f"{fid}-{kind}.html")
            open(page, "w", encoding="utf-8").write(f"<!doctype html><html><head><meta charset='utf-8'><style>{css}\nhtml,body{{margin:0;background:#fff}}svg{{display:block}}</style></head><body>{svg}</body></html>")
            png = os.path.join(work, f"{fid}-{kind}.png")
            subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1", "--window-size=1920,1080", "--virtual-time-budget=4000", f"--screenshot={png}", "file:///" + page.replace("\\", "/")], capture_output=True, timeout=120)
            pngs[(fid, kind)] = png
    tw = spec["sheet"]["contact"]
    th = round(tw * 1080 / 1920)
    gap, label, head = spec["sheet"]["contact_gap"], spec["sheet"]["contact_label"], spec["sheet"]["contact_head"]
    cols = 4
    rows = len(LAYOUTS) * 2 // cols
    W = cols * tw + (cols + 1) * gap
    H = head + rows * (th + label) + (rows + 1) * gap
    sheet = Image.new("RGB", (W, H), (28, 28, 30))
    d = ImageDraw.Draw(sheet)
    bold, reg = ImageFont.truetype(r"C:\Windows\Fonts\arialbd.ttf", 26), ImageFont.truetype(r"C:\Windows\Fonts\arial.ttf", 21)
    d.text((gap, 14), "Artosia announcement template library: eight families, birthdays (30 names, 7 highlighted) and offerings (6 funds, 12-digit amounts), default colourway, 1920x1080 wireframes", font=bold, fill=(240, 240, 240))
    d.text((gap, 46), "Drawn from spec.json by build/build.py; rendered to PNG by headless Google Chrome 157 from the SVGs; photos and glows are stand-ins.", font=reg, fill=(180, 180, 185))
    for i, ((fid, kind), png) in enumerate(pngs.items()):
        r, c = divmod(i, cols)
        x, y = gap + c * (tw + gap), head + gap + r * (th + label + gap)
        sheet.paste(Image.open(png).convert("RGB").resize((tw, th), Image.LANCZOS), (x, y))
        fam = spec["families"][fid]
        d.text((x, y + th + 6), f"{fam['no']}  {fam['name']}  ·  {fam['colourways'][0]['name']}", font=bold, fill=(245, 245, 245))
        d.text((x, y + th + 36), f"{kind} {'birthdays' if kind == 'S02' else 'offerings'}  ·  {fam['lane']}", font=reg, fill=(190, 190, 195))
    target = os.path.join(OUT, "contact-sheet.png")
    sheet.save(target, optimize=True)
    print("render:", target, sheet.size, os.path.getsize(target), "bytes")


# ------------------------------------------------------------------ doc

def table(head, rows):
    return "\n".join(["| " + " | ".join(head) + " |", "|" + "|".join("---" for _ in head) + "|"] + ["| " + " | ".join(str(c) for c in r) + " |" for r in rows])


def gen_type(spec, fid):
    fam = spec["families"][fid]
    fig = spec["generated"]["figures"][fid]
    fonts = table(["key", "family", "weight", "file", "source", "figures"], [[k, v[0], f"{v[1]}{' italic' if v[2] else ''}", v[3] + ".ttf", fam["font_source"][k], fig[k]] for k, v in fam["fonts"].items()])
    H = spec["sheet"]["h"]
    rows = [[role, fam["fonts"][f][0] + " " + str(fam["fonts"][f][1]) + (" i" if fam["fonts"][f][2] else ""), sz, f"{sz / H * 100:.1f}%", ls if ls else "0", "caps" if caps else "as typed", c] for role, (f, sz, ls, caps, c) in fam["type"].items()]
    return fonts + "\n\n" + table(["role", "face", "px on 1080", "% of height", "tracking em", "case", "colour"], rows)


def gen_colour(spec, fid):
    fam = spec["families"][fid]
    out = []
    keys = list(fam["colourways"][0]["palette"])
    out.append(table(["role"] + [f"{cw['id']} ({cw['name']})" for cw in fam["colourways"]], [[k] + [f"`{cw['palette'][k]}`" for cw in fam["colourways"]] for k in keys]))
    if fam.get("layers"):
        out.append("Layers (a translucent fill over its ground; contrast is measured on the composite, over a black and a white photo pixel where it sits on a photo): " + "; ".join(f"`{k}` = {v['fill']} at {v['op']} over {v['over']}" for k, v in fam["layers"].items()))
    con = spec["generated"]["contrast"][fid]
    rows = []
    for i, (fg, bg, kind) in enumerate(fam["pairs"]):
        rows.append([f"{fg} on {bg}", kind, fam["pair_use"][i]] + [f"{con[cw['id']][i][3]:.2f}" + ("" if con[cw["id"]][i][4] else " FAIL") for cw in fam["colourways"]])
    out.append(table(["text on ground", "needs", "where it is drawn"] + [cw["id"] for cw in fam["colourways"]], rows))
    least = {cw["id"]: min(r[3] for r in con[cw["id"]]) for cw in fam["colourways"]}
    out.append(f"Gallery swatch: ground `{fam['swatch'][0]}`, accent `{fam['swatch'][1]}` (the `swatch` keys of the tokens file).")
    out.append("Least ratio per colourway: " + ", ".join(f"{k} {v:.2f}" for k, v in least.items()) + f". Needs: text {spec['thresholds']['text']}, large text and marks {spec['thresholds']['large']}.")
    return "\n\n".join(out)


def gen_mock(spec, fid):
    """The mock's measurable values, for the builder to set its measured values beside (the last two columns are the builder's)."""
    fam = spec["families"][fid]
    m, T, pal = fam["mock"], fam["tokens"], fam["colourways"][0]["palette"]
    s2, s3 = spec["generated"]["scenes"][f"{fid}/S02"]["fits"], spec["generated"]["scenes"][f"{fid}/S03"]["fits"]
    rows = [
        ["title size (px, 1080 sheet)", fam["type"]["title"][1], "2 px"],
        ["birthday name size at 30 names (px)", s2["name_size"], "1 px"],
        ["birthday row pitch (px)", s2["pitch"], "2 px"],
        ["date column (px)", s2["day_col"], "4 px"],
        ["name cell (px)", s2["name_cell"], "4 px"],
        ["names shortened of 30", s2["names_shortened"], "exact"],
        ["offering row pitch (px)", s3["pitch"], "2 px"],
        ["amount column, widest 12-digit amount with Rp (px)", s3["amount_col"], "4 px"],
        ["total figure width (px)", s3["total_px"], "4 px"],
        ["left margin (px)", T[m["margin"]], "1 px"],
        ["corner radius (px)", T[m["radius"]] if m.get("radius") else "none", "1 px"],
        ["ground", pal[m["ground"]], "exact"],
        ["accent", pal[m["accent"]], "exact"],
    ]
    return table(["value", "mock", "tolerance", "built", "measured at"], [r + ["", ""] for r in rows])


def gen_head(spec, fid):
    f = spec["families"][fid]
    refs = "\n".join(f"- `{p}`: {w}" for p, w in f["refs"])
    orn = "; ".join(f"{t} (`{r}`)" for t, r in f["ornaments"])
    return f"**Idea.** {f['idea']}\n\n**References it copies** (under `C:\\Users\\kenne\\Shared\\inspo\\`):\n{refs}\n\n**Ornaments** (only those the references show): {orn}."


def gen_grid(spec, fid):
    fam = spec["families"][fid]
    return table(["token", "value", "what it sets"], [[k, fam["tokens"][k] if not isinstance(fam["tokens"][k], list) else "list", d] for k, d in fam["grid"]])


def gen_kinds(spec, fid):
    k = spec["generated"]["kinds"][fid]
    a, b, c, d = k["s01"], k["s04"], k["s05"], k["s06"]
    rows = [
        ["S01 schedule", f"{a['rows']} rows over {a['days']} days, {a['special']} special", f"event {a['size']} px (role {a['role_size']}, height allows {a['size_by_height']}); longest event '{a['longest_event']}' is {a['longest_px']} px in a {a['event_w']} px cell: {a['lines']} line{'s' if a['lines'] > 1 else ''}"],
        ["S04 account", f"{b['digits']}-digit number", f"with QR: {b['with_qr']['size']} px on {b['with_qr']['lines']} line{'s' if b['with_qr']['lines'] > 1 else ''} in {b['with_qr']['width']} px; without: {b['without_qr']['size']} px on {b['without_qr']['lines']} line{'s' if b['without_qr']['lines'] > 1 else ''} in {b['without_qr']['width']} px"],
        ["S05 verse", f"{c['words']} words (width x{INDONESIAN_LONGER})", f"with picture {c['with_picture']['size']} px in {c['with_picture']['lines']} lines ({c['with_picture']['height']} of {c['with_picture']['room']} px); without {c['without_picture']['size']} px in {c['without_picture']['lines']} lines ({c['without_picture']['height']} of {c['without_picture']['room']} px)"],
        ["S06 notice", "a 45-character title", f"with picture {d['with_picture']['size']} px on {d['with_picture']['lines']} lines (role {d['with_picture']['role_size']}); without {d['without_picture']['size']} px on {d['without_picture']['lines']} lines"],
    ]
    return table(["kind", "worst case", "measured with this family's faces"], rows)


def gen_foot(spec):
    rows = [[f["no"] + " " + f["name"], f["foot"]["x"], f["foot"]["cy"], f["foot"]["a"], f["foot"]["c"]] for f in spec["families"].values()]
    return table(["family", "x", "vertical middle", "anchor", "colour"], rows)


def gen_compare(spec):
    rows = [[f["no"], f["name"], f["lane"], f["compare"]["ground"], f["compare"]["skeleton"], f["compare"]["type"], f["compare"]["highlight"], f["compare"]["total"], f["compare"]["ornament"], f["compare"]["needs"]] for f in spec["families"].values()]
    return table(["#", "family", "pull", "ground and colour logic", "layout skeleton", "type voice", "highlight", "total", "ornament", "engine needs"], rows)


def gen_ledger(spec):
    return table(["id", "source", "owner item", "honoured by"], [[r["id"], r["source"], r["item"], r.get("dropped") and "dropped: " + r["dropped"] or ", ".join(r["maps_to"])] for r in spec["ledger"]])


def gen_fits(spec):
    rows = []
    for fid in LAYOUTS:
        a, b = spec["generated"]["scenes"][f"{fid}/S02"]["fits"], spec["generated"]["scenes"][f"{fid}/S03"]["fits"]
        alt = a.get("shortened_if_each_row_names_its_month")
        rows.append([spec["families"][fid]["name"], "month heading per column" if alt is not None else "month on every row", a["name_size"], a["names_shortened"], alt if alt is not None else "-", a["name_cell"], a["longest_name_px"], b["label_lines"].count(2), b["amount_col"], b["total_px"]])
    return table(["family", "date rule used", "name px", "names shortened of 30", "if every row named its month", "name cell px", "longest full name px", "fund labels on two lines of 6", "amount column px", "total figure px"], rows)


def doc():
    spec = load()
    text = open(MD, encoding="utf-8").read()
    makers = {"compare": lambda: gen_compare(spec), "ledger": lambda: gen_ledger(spec), "fits": lambda: gen_fits(spec), "foot": lambda: gen_foot(spec)}
    for fid in LAYOUTS:
        makers[f"type:{fid}"] = lambda f=fid: gen_type(spec, f)
        makers[f"colour:{fid}"] = lambda f=fid: gen_colour(spec, f)
        makers[f"mock:{fid}"] = lambda f=fid: gen_mock(spec, f)
        makers[f"grid:{fid}"] = lambda f=fid: gen_grid(spec, f)
        makers[f"kinds:{fid}"] = lambda f=fid: gen_kinds(spec, f)
        makers[f"head:{fid}"] = lambda f=fid: gen_head(spec, f)
    done = 0
    for key, make in makers.items():
        pat = re.compile(rf"(<!-- gen:{re.escape(key)} -->\n).*?(\n<!-- /gen:{re.escape(key)} -->)", re.S)
        if pat.search(text):
            text = pat.sub(lambda m: m.group(1) + make() + m.group(2), text)
            done += 1
    open(MD, "w", encoding="utf-8", newline="\n").write(text)
    print(f"doc: {done} generated blocks filled in {MD}")


if __name__ == "__main__":
    step = sys.argv[1]
    if step == "layout":
        layout(sys.argv[2])
    elif step == "draw":
        draw()
    elif step == "check":
        check()
    elif step == "render":
        render(sys.argv[2], sys.argv[3])
    elif step == "doc":
        doc()
