"""Colour helpers: OKLCH to hex with gamut mapping, and the contrast ratio exactly as crates/artosia-slides/src/look.rs computes it (WCAG 2, threshold 0.03928)."""
import math


def _lin(c):
    return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4


def luminance(hexs):
    r, g, b = (int(hexs[i:i + 2], 16) / 255 for i in (1, 3, 5))
    return 0.2126 * _lin(r) + 0.7152 * _lin(g) + 0.0722 * _lin(b)


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def _enc(c):
    c = max(0.0, min(1.0, c))
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def oklch_to_rgb(L, C, H):
    a = C * math.cos(math.radians(H))
    b = C * math.sin(math.radians(H))
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    bl = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    return r, g, bl


def in_gamut(rgb):
    return all(-1e-4 <= v <= 1 + 1e-4 for v in rgb)


def hex_of(L, C, H):
    """Reduce chroma until the colour fits sRGB, keep lightness and hue."""
    c = C
    while c > 0 and not in_gamut(oklch_to_rgb(L, c, H)):
        c -= 0.002
    r, g, b = oklch_to_rgb(L, max(c, 0), H)
    return "#%02X%02X%02X" % tuple(round(_enc(v) * 255) for v in (r, g, b))


def to_oklch(hexs):
    r, g, b = (_lin(int(hexs[i:i + 2], 16) / 255) for i in (1, 3, 5))
    # the OKLab matrices use the exact sRGB transfer, close enough to 0.03928 for design work
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l, m, s = l ** (1 / 3), m ** (1 / 3), s ** (1 / 3)
    L = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s
    a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s
    bb = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s
    return L, math.hypot(a, bb), math.degrees(math.atan2(bb, a)) % 360


def delta_e(h1, h2):
    """OKLab distance between two hex colours."""
    def lab(h):
        L, C, H = to_oklch(h)
        return L, C * math.cos(math.radians(H)), C * math.sin(math.radians(H))
    a, b = lab(h1), lab(h2)
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)))
