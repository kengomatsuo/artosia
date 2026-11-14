"""Shared helpers of the library wireframes: spec file, font measuring, WCAG contrast, the scene a family draws into.

Everything a drawing shows is written into spec.json under "generated" by `build.py layout`; `build.py draw` reads only
that file, so a number that lives only in drawing code cannot exist (build.py check prints any that does).
"""
import json, os, re
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.dirname(HERE)
SPEC = os.path.join(OUT, "spec.json")
MARK = '\n"generated":'
HONORIFICS = ("Bpk.", "Ibu", "Sdri.", "Sdr.", "Ny.", "Adk.")


def load():
    return json.load(open(SPEC, encoding="utf-8"))


def save_generated(generated):
    """Replace the "generated" block at the end of spec.json; the authored text above it is never touched."""
    text = open(SPEC, encoding="utf-8").read()
    cut = text.find(MARK)
    head = (text[:cut] if cut >= 0 else text.rstrip().rstrip("}")).rstrip().rstrip(",")
    body = ",\n".join(f' {json.dumps(k)}: {json.dumps(v, ensure_ascii=False, separators=(",", ":"))}' for k, v in generated.items())
    open(SPEC, "w", encoding="utf-8", newline="\n").write(head + ",\n" + MARK.lstrip("\n") + " {\n" + body + "\n}\n}\n")


def r2(v):
    """Round a number to two decimals, whole numbers stay whole."""
    v = round(float(v), 2)
    return int(v) if v == int(v) else v


# ---------------------------------------------------------------- fonts

class Fonts:
    def __init__(self, folder):
        self.folder, self.cache = folder, {}

    def face(self, stem):
        if stem not in self.cache:
            t = TTFont(os.path.join(self.folder, stem + ".ttf"))
            upm = t["head"].unitsPerEm
            cap = getattr(t["OS/2"], "sCapHeight", 0) or 0.7 * upm
            self.cache[stem] = (t.getBestCmap(), t["hmtx"], upm, cap)
        return self.cache[stem]

    def width(self, stem, text, size, ls=0.0):
        cmap, hmtx, upm, _ = self.face(stem)
        total = sum(hmtx[cmap[ord(c)]][0] if ord(c) in cmap else upm * 0.5 for c in text)
        return total * size / upm + ls * size * len(text)

    def cap(self, stem, size):
        _, _, upm, cap = self.face(stem)
        return cap * size / upm

    def covers(self, stem, text):
        cmap = self.face(stem)[0]
        return sorted({c for c in text if not c.isspace() and ord(c) not in cmap})


# ---------------------------------------------------------------- contrast

def rgb(code):
    return tuple(int(code[i:i + 2], 16) for i in (1, 3, 5))


def lum(c):
    def lin(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(v) for v in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = lum(a), lum(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def over(fg, alpha, bg):
    return tuple(round(fg[i] * alpha + bg[i] * (1 - alpha)) for i in range(3))


def grounds(palette, layers, name):
    """The colours a ground can really be: a palette key, or a layer over the ground or over any photo pixel (black and white)."""
    if name in palette:
        return [rgb(palette[name])]
    layer = layers[name]
    top, a = rgb(palette[layer["fill"]]), layer["op"]
    under = [(0, 0, 0), (255, 255, 255)] if layer["over"] == "photo" else [rgb(palette[layer["over"]])]
    return [over(top, a, u) for u in under]


def pair_ratio(palette, layers, fg, bg):
    """The least ratio of `fg` on `bg` over every colour `bg` can be."""
    fgs = grounds(palette, layers, fg) if fg not in palette and fg in layers else [rgb(palette[fg])]
    return min(ratio(f, g) for f in fgs for g in grounds(palette, layers, bg))


# ---------------------------------------------------------------- names

def initials(word):
    return word[0].upper() + "."


def ladder(name, fits):
    """The longest form of a name `fits(text)` accepts, from full to all initials; (text, level) or None.

    0 full; 1 middle names as initials; 2 every name but the surname as an initial; 3 first initial and surname;
    4 initials only (the engine's last resort).
    """
    words = name.split()
    hon = words[0] if words and words[0] in HONORIFICS else None
    rest = words[1:] if hon else words
    head = [hon] if hon else []
    forms = [name]
    if len(rest) > 2:
        forms.append(" ".join(head + [rest[0]] + [initials(w) for w in rest[1:-1]] + [rest[-1]]))
    if len(rest) > 1:
        forms.append(" ".join(head + [initials(w) for w in rest[:-1]] + [rest[-1]]))
        forms.append(" ".join(head + [initials(rest[0]), rest[-1]]))
        forms.append(" ".join(head + [initials(w) for w in rest]))
    for form in dict.fromkeys(forms):
        if fits(form):
            return form, forms.index(form)
    return None


# ---------------------------------------------------------------- scene

class Scene:
    """Elements of one slide, every number resolved. Colours are palette keys; fonts are the family's font keys."""

    def __init__(self, fam, fonts, W, H):
        self.fam, self.fonts, self.W, self.H = fam, fonts, W, H
        self.els, self.fits = [], {}

    # fonts and type roles
    def stem(self, key):
        return self.fam["fonts"][key][3]

    def role(self, name):
        key, size, ls, caps, colour = self.fam["type"][name]
        return key, size, ls, caps, colour

    def w(self, role, s, size=None, font=None):
        key, sz, ls, caps, _ = self.role(role)
        return self.fonts.width(self.stem(font or key), s.upper() if caps else s, size or sz, ls)

    def cap(self, role, size=None, font=None):
        key, sz, *_ = self.role(role)
        return self.fonts.cap(self.stem(font or key), size or sz)

    # primitives
    def add(self, el):
        def deep(v):
            if isinstance(v, float):
                return r2(v)
            if isinstance(v, list):
                return [deep(i) for i in v]
            return v
        self.els.append({k: deep(v) for k, v in el.items() if v is not None})
        return el

    def rect(self, x, y, w, h, fill=None, op=None, rx=None, stroke=None, sw=None, rot=None, dash=None):
        return self.add(dict(k="r", x=r2(x), y=r2(y), w=r2(w), h=r2(h), c=fill, op=op, rx=rx, st=stroke, sw=sw, rot=rot,
                             rcx=r2(x + w / 2) if rot else None, rcy=r2(y + h / 2) if rot else None, dash=dash))

    def poly(self, pts, fill, op=None):
        return self.add(dict(k="p", pts=[[r2(a), r2(b)] for a, b in pts], c=fill, op=op))

    def circle(self, cx, cy, r, fill=None, op=None, stroke=None, sw=None):
        return self.add(dict(k="o", cx=r2(cx), cy=r2(cy), r=r2(r), c=fill, op=op, st=stroke, sw=sw))

    def line(self, x1, y1, x2, y2, stroke, sw, op=None, dash=None):
        return self.add(dict(k="l", x1=r2(x1), y1=r2(y1), x2=r2(x2), y2=r2(y2), st=stroke, sw=sw, op=op, dash=dash))

    def glow(self, cx, cy, r, stops):
        """A radial glow: stops are [offset, colour key, opacity]."""
        return self.add(dict(k="g", cx=cx, cy=cy, r=r, stops=stops))

    def photo(self, x, y, w, h, tint_a, tint_b, blobs):
        """The picture placeholder: a vertical gradient with soft blobs, standing for the church's own photo."""
        # each blob: centre, radii, opacity, blur, then the filter region that holds it
        full = [[cx, cy, rx, ry, op, blur, cx - rx - 3 * blur, cy - ry - 3 * blur, 2 * rx + 6 * blur, 2 * ry + 6 * blur] for cx, cy, rx, ry, op, blur in blobs]
        return self.add(dict(k="f", x=x, y=y, w=w, h=h, a=tint_a, b=tint_b, blobs=full))

    def text(self, role, s, x, cy=None, base=None, anchor="start", fill=None, size=None, font=None, op=None, rot=None, caps=None):
        """One line of text. `cy` is the vertical middle of its capitals; `base` is the baseline itself."""
        key, sz, ls, cp, colour = self.role(role)
        size = size or sz
        fkey = font or key
        s = s.upper() if (cp if caps is None else caps) else s
        if base is None:
            base = cy + self.fonts.cap(self.stem(fkey), size) / 2
        # letter-spacing trails the last glyph, so an end or middle anchor moves back by it
        trail = ls * size
        x = x + (trail if anchor == "end" else trail / 2 if anchor == "middle" else 0)
        self.add(dict(k="t", s=s, x=r2(x), y=r2(base), a=anchor, f=fkey, z=size, ls=ls or None, c=fill or colour, op=op, rot=rot))
        return self.fonts.width(self.stem(fkey), s, size, ls)

    def lines(self, role, ls_text, x, cy, pitch, **kw):
        for i, s in enumerate(ls_text):
            self.text(role, s, x, cy + i * pitch, **kw)


def day_of(copy, n):
    """A name's day as the engine writes it on a page across two months: "29 Nov", "01 Des"."""
    return f"{n['d']:02d} {copy['months_short'][n['m'] - 1]}"


def rupiah(n):
    return f"{n:,}".replace(",", ".")


def wrap(fonts, stem, text, size, width, ls=0.0):
    """Greedy wrap into lines no wider than `width`; one line when it fits."""
    lines, cur = [], ""
    for word in text.split():
        trial = (cur + " " + word).strip()
        if cur and fonts.width(stem, trial, size, ls) > width:
            lines.append(cur)
            cur = word
        else:
            cur = trial
    return lines + [cur]


def balanced(fonts, stem, text, size, width, ls=0.0):
    """The greedy wrap narrowed to the least width that keeps its line count: lines of near-equal length."""
    count = len(wrap(fonts, stem, text, size, width, ls))
    lo, hi = width * 0.5, width
    for _ in range(18):
        mid = (lo + hi) / 2
        if len(wrap(fonts, stem, text, size, mid, ls)) <= count:
            hi = mid
        else:
            lo = mid
    return wrap(fonts, stem, text, size, hi, ls)


def numbers_in(value, out):
    """Every number anywhere in a JSON value, rounded to two decimals."""
    if isinstance(value, bool):
        return
    if isinstance(value, (int, float)):
        out.add(round(float(value), 2))
    elif isinstance(value, dict):
        for v in value.values():
            numbers_in(v, out)
    elif isinstance(value, list):
        for v in value:
            numbers_in(v, out)


NUM = re.compile(r"-?\d+(?:\.\d+)?")
