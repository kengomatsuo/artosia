"""Shared engine: text measure with the real faces, the display list a slide is, the name ladder and fit helpers, contrast, the SVG writer.

A slide is a display list: rect and text items with every number explicit. It lives in spec.json under "resolved".
The SVG writer reads only that list and the colourway's palette, and adds no number of its own.
"""
import base64, io, json, os, re
import uharfbuzz as hb
from fontTools.ttLib import TTFont
from fontTools import subset

W, H = 1920, 1080
FONT_DIR = os.environ.get("ARTOSIA_FONTS", "")  # FONT_DIR/static holds the cuts make_fonts.py wrote


def static_dir():
    return os.path.join(FONT_DIR, "static")


# ---------------------------------------------------------------- faces and measure
class Face:
    _cache = {}

    def __init__(self, file):
        self.path = os.path.join(static_dir(), file)
        self.hb = hb.Font(hb.Face(hb.Blob.from_file_path(self.path)))
        tt = TTFont(self.path)
        self.upem = tt["head"].unitsPerEm
        self.cap = tt["OS/2"].sCapHeight / self.upem
        self.memo = {}

    @classmethod
    def of(cls, file):
        if file not in cls._cache:
            cls._cache[file] = Face(file)
        return cls._cache[file]

    def adv(self, s, size, ls=0.0):
        """Advance of s at size px with letter-spacing ls px between letters."""
        if s not in self.memo:
            buf = hb.Buffer()
            buf.add_str(s)
            buf.guess_segment_properties()
            hb.shape(self.hb, buf, {"kern": True, "liga": True})
            self.memo[s] = sum(p.x_advance for p in buf.glyph_positions) / self.upem
        return self.memo[s] * size + ls * len(s)


def lum(hexc):
    h = hexc.lstrip("#")
    c = [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    c = [v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4 for v in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def contrast(fg, bg):
    """WCAG ratio, the formula of crates/artosia-slides/src/look.rs contrast_of."""
    a, b = lum(fg), lum(bg)
    return (max(a, b) + 0.05) / (min(a, b) + 0.05)


def mix(a, b, t):
    """Colour a over colour b at alpha t."""
    ca = [int(a.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    cb = [int(b.lstrip("#")[i:i + 2], 16) for i in (0, 2, 4)]
    return "#" + "".join(f"{round(ca[i] * t + cb[i] * (1 - t)):02X}" for i in range(3))


def digits(n):
    """12345678 -> '12.345.678', the church's one style (shared.json money)."""
    return f"{n:,}".replace(",", ".")


HONORIFICS = ("Bpk.", "Ibu", "Sdr.", "Sdri.", "Ny.", "Adk.")


def initials(name):
    """fit.rs initials: middle names to their initial, the honorific kept."""
    parts = name.split()
    head = 1 if parts and parts[0] in HONORIFICS else 0
    rest = parts[head:]
    if len(rest) <= 2:
        return name
    return " ".join(parts[:head] + [rest[0]] + [p[0] + "." for p in rest[1:-1]] + [rest[-1]])


def largest(hi, lo, ok, step=1):
    """Largest size from hi down to lo at which ok holds, or None (fit.rs largest_step)."""
    z = hi
    while z >= lo:
        if ok(z):
            return z
        z -= step
    return None


def wrap(words, width, space, wmeasure):
    lines, cur, used = [], [], 0.0
    for w in words:
        wd = wmeasure(w)
        if cur and used + space + wd > width:
            lines.append(cur)
            cur, used = [w], wd
        else:
            used += (space if cur else 0) + wd
            cur.append(w)
    if cur:
        lines.append(cur)
    return lines


def balanced(words, width, space, wmeasure):
    """The greedy wrap narrowed to the least width that keeps its line count (fit.rs balanced)."""
    count = len(wrap(words, width, space, wmeasure))
    lo, hi = width * 0.5, width
    for _ in range(18):
        mid = (lo + hi) / 2
        if len(wrap(words, mid, space, wmeasure)) <= count:
            hi = mid
        else:
            lo = mid
    return wrap(words, hi, space, wmeasure)


# ---------------------------------------------------------------- the slide
class Sheet:
    """One slide of one candidate in one colourway: tokens T, data D, and the display list items."""

    def __init__(self, T, D, cw="light"):
        self.T, self.D = T, D
        self.cw = cw
        self.P = next(c["palette"] for c in T["colourways"] if c["id"] == cw)
        self.items = []
        self.fit = {}

    # tokens
    def g(self, key):
        return self.T["grid"][key]

    def k(self, section):
        s = self.T[section]
        return lambda key: s[key]

    def role(self, name):
        return self.T["type"][name]

    def font(self, key):
        return self.T["fonts"][key]

    def face(self, role, font=None):
        return Face.of(self.font(font or self.role(role)["font"])["file"])

    def size_of(self, role):
        return self.role(role)["size"]

    def ls_px(self, role, size):
        return self.role(role).get("ls", 0.0) * size

    def cap(self, role, size=None):
        return self.face(role).cap * (size or self.size_of(role))

    def measure(self, role, s, size=None, font=None):
        r = self.role(role)
        size = size or r["size"]
        s = s.upper() if r.get("caps") else s
        return self.face(role, font).adv(s, size, self.ls_px(role, size))

    # drawing
    def rect(self, x, y, w, h, fill=None, op=1.0, stroke=None, sw=0.0, rx=0.0, grad=None, id=""):
        it = {"k": "rect", "id": id, "x": round(x, 2), "y": round(y, 2), "w": round(w, 2), "h": round(h, 2), "rx": rx, "fill": fill, "op": op, "stroke": stroke, "sw": sw}
        if grad:
            it["grad"] = grad
        self.items.append(it)
        return it

    def text(self, role, s, x, base, anchor="l", size=None, fill=None, font=None, id="", ls=None):
        r = self.role(role)
        size = size or r["size"]
        s = s.upper() if r.get("caps") else s
        fk = font or r["font"]
        f = self.font(fk)
        lsp = self.ls_px(role, size) if ls is None else ls
        w = Face.of(f["file"]).adv(s, size, lsp)
        x0 = x if anchor == "l" else (x - w if anchor == "r" else x - w / 2)
        it = {"k": "text", "id": id, "role": role, "t": s, "x": round(x0, 2), "y": round(base, 2), "w": round(w, 2), "size": size, "ls": round(lsp, 3),
              "fill": fill or r["color"], "family": f["family"], "weight": f["weight"], "italic": f["italic"]}
        self.items.append(it)
        return it

    def need(self, ok, why=""):
        if not ok:
            raise Split(why)


class Split(Exception):
    """The layout does not hold at its floor: the card continues on a second slide."""


def ladder(sheet, role, name, width, size):
    """The name whole, else with middle names as initials; None when neither fits width."""
    for cand in (name, initials(name)):
        if sheet.measure(role, cand, size) <= width:
            return cand
    return None


# ---------------------------------------------------------------- SVG
def used_chars(items):
    out = {}
    for it in items:
        if it["k"] == "text":
            out.setdefault((it["family"], it["weight"], it["italic"]), set()).update(it["t"])
    return out


def font_file(family, weight, italic):
    return f"{family.replace(' ', '')}-{weight}{'i' if italic else ''}.ttf"


def embed_css(items):
    """Subset every face a slide uses to its characters and embed it, so the SVG shows the same type in any browser."""
    css = []
    for (family, weight, italic), chars in sorted(used_chars(items).items()):
        path = os.path.join(static_dir(), font_file(family, weight, italic))
        opts = subset.Options()
        opts.layout_features = ["*"]
        opts.name_IDs = ["*"]
        opts.name_languages = ["*"]
        opts.notdef_outline = True
        opts.glyph_names = False
        opts.hinting = False
        opts.legacy_kern = True
        font = TTFont(path)
        sub = subset.Subsetter(opts)
        sub.populate(text="".join(sorted(chars)))
        sub.subset(font)
        buf = io.BytesIO()
        font.save(buf)
        b64 = base64.b64encode(buf.getvalue()).decode()
        css.append(f'@font-face{{font-family:"{family}";font-weight:{weight};font-style:{"italic" if italic else "normal"};src:url(data:font/ttf;base64,{b64}) format("truetype")}}')
    return "".join(css)


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def num(v):
    return f"{v:g}"


def write_svg(T, cw, sid, items, title, embed=True):
    P = next(c["palette"] for c in T["colourways"] if c["id"] == cw)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">', f"<title>{esc(title)}</title>"]
    grads = {it["grad"]["name"]: it["grad"] for it in items if it.get("grad")}
    if embed:
        out.append(f"<style>{embed_css(items)}</style>")
    if grads:
        out.append("<defs>")
        for name, g in grads.items():
            if g["kind"] == "radial":
                out.append(f'<radialGradient id="grad-{name}" gradientUnits="userSpaceOnUse" cx="{num(g["cx"])}" cy="{num(g["cy"])}" r="{num(g["r"])}">')
            else:
                out.append(f'<linearGradient id="grad-{name}" x1="0" y1="0" x2="0" y2="1">')
            for off, key, a in g["stops"]:
                out.append(f'<stop offset="{num(off)}" stop-color="{P[key]}" stop-opacity="{num(a)}"/>')
            out.append("</radialGradient>" if g["kind"] == "radial" else "</linearGradient>")
        out.append("</defs>")
    for it in items:
        if it["k"] == "rect":
            fill = f'url(#grad-{it["grad"]["name"]})' if it.get("grad") else (P[it["fill"]] if it["fill"] else "none")
            a = [f'x="{num(it["x"])}"', f'y="{num(it["y"])}"', f'width="{num(it["w"])}"', f'height="{num(it["h"])}"']
            if it["rx"]:
                a.append(f'rx="{num(it["rx"])}"')
            a.append(f'fill="{fill}"')
            if it["op"] != 1:
                a.append(f'opacity="{num(it["op"])}"')
            if it["stroke"]:
                a += [f'stroke="{P[it["stroke"]]}"', f'stroke-width="{num(it["sw"])}"']
            out.append(f'<rect id="{it["id"]}" ' + " ".join(a) + "/>")
        else:
            a = [f'x="{num(it["x"])}"', f'y="{num(it["y"])}"', f'font-family="{it["family"]}"', f'font-weight="{it["weight"]}"']
            if it["italic"]:
                a.append('font-style="italic"')
            a += [f'font-size="{num(it["size"])}"']
            if it["ls"]:
                a.append(f'letter-spacing="{num(it["ls"])}"')
            a.append(f'fill="{P[it["fill"]]}"')
            out.append(f'<text id="{it["id"]}" ' + " ".join(a) + f'>{esc(it["t"])}</text>')
    out.append("</svg>")
    return "\n".join(out)
