"""The season marks: sixteen glyphs on a 24-unit grid, one line style.

Style (written into spec.json "grid"): stroke 2 units, round caps and joins, fill none; a shape smaller than 5 units across, and a light
(flame, star, spark), is drawn solid in the same colour. Every colour is currentColor. Elements: path, rect, circle only, the subset the
icon painter of Rune accepts (crates/rune/src/text/icon.rs): M L H V C Q A Z, groups that carry the paint attributes.

This file computes path data once; build.py writes it into spec.json, and every drawing reads it from there.
"""
import math


def f(x):
    s = ("%.2f" % x).rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def pt(x, y):
    return "%s %s" % (f(x), f(y))


def poly(pts, close=True):
    return "M" + "L".join(pt(x, y) for x, y in pts) + ("Z" if close else "")


def polar(cx, cy, a, r):
    return (cx + r * math.sin(math.radians(a)), cy - r * math.cos(math.radians(a)))


def teardrop(cx, top, w, h):
    r = w / 2
    cyb = top + h - r
    return "M%s %sC%s %s %s %s %s %sA%s %s 0 0 1 %s %sC%s %s %s %s %s %sZ" % tuple(map(f, (
        cx, top, cx + w * 0.10, top + h * 0.26, cx + r, top + h * 0.46, cx + r, cyb, r, r, cx - r, cyb,
        cx - r, top + h * 0.46, cx - w * 0.10, top + h * 0.26, cx, top)))


# A flame with a lick on its left side (Pentecost), drawn in a 10 by 16 box and scaled.
_FLAME = [("M", 1.2, 0), ("C", 1.6, 3.2, 5, 5.2, 5, 10), ("C", 5, 13.6, 2.8, 16, 0, 16), ("C", -2.8, 16, -5, 13.6, -5, 10.4),
          ("C", -5, 8, -3.6, 6.6, -2.8, 5.4), ("C", -2.6, 7, -1.8, 7.8, -1, 8), ("C", -1.2, 5, -0.6, 2, 1.2, 0), ("Z",)]


def flame(cx, y0, w, h):
    out = ""
    for seg in _FLAME:
        out += seg[0]
        nums = seg[1:]
        out += " ".join("%s %s" % (f(nums[i] * w / 10 + cx), f(nums[i + 1] * h / 16 + y0)) for i in range(0, len(nums), 2))
    return out


def sparkle(cx, cy, rx, ry, p):
    c = lambda *a: " ".join(f(v) for v in a)
    return "M%sQ%s %sQ%s %sQ%s %sQ%s %s Z" % (c(cx, cy - ry), c(cx + p, cy - p), c(cx + rx, cy), c(cx + p, cy + p), c(cx, cy + ry), c(cx - p, cy + p), c(cx - rx, cy), c(cx - p, cy - p), c(cx, cy - ry))


def bez(p0, p1, p2, p3, t):
    u = 1 - t
    return tuple(u ** 3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t ** 3 * d for a, b, c, d in zip(p0, p1, p2, p3))


def bezt(p0, p1, p2, p3, t):
    u = 1 - t
    return tuple(3 * u * u * (b - a) + 6 * u * t * (c - b) + 3 * t * t * (d - c) for a, b, c, d in zip(p0, p1, p2, p3))


def shift_path(d, dx, dy):
    """Move absolute path data by (dx, dy): M L H V C Q A Z only."""
    import re
    out = ""
    for cmd, args in re.findall(r"([MLHVCQAZ])([^MLHVCQAZ]*)", d):
        nums = [float(n) for n in re.findall(r"-?\d+\.?\d*", args)]
        if cmd in "ML" or cmd in "CQ":
            nums = [v + (dx if i % 2 == 0 else dy) for i, v in enumerate(nums)]
        elif cmd == "H":
            nums = [v + dx for v in nums]
        elif cmd == "V":
            nums = [v + dy for v in nums]
        elif cmd == "A":
            nums = [nums[0], nums[1], nums[2], nums[3], nums[4], nums[5] + dx, nums[6] + dy]
        out += cmd + " ".join(f(v) for v in nums)
    return out


def line(d):
    return '<path d="%s"/>' % d


def solid(d):
    return '<path d="%s" fill="currentColor" stroke="none"/>' % d


def solid_round(d):
    return '<path d="%s" fill="currentColor" stroke="currentColor" stroke-width="1"/>' % d


def candle(x, y, w, h):
    return '<rect x="%s" y="%s" width="%s" height="%s" rx="%s" fill="currentColor" stroke="none"/>' % (f(x), f(y), f(w), f(h), f(min(w / 2, 1.2)))


def wrap(inner):
    return '<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">%s</g>' % inner


def advent_row(n):
    out = ""
    for i, x in enumerate([2.2, 7.6, 13.0, 18.4]):
        out += candle(x, 9.0, 3.4, 8.5)
        if i < n:
            out += solid(teardrop(x + 1.7, 2.5, 3.4, 5.2))
    return out + line("M3 19.5Q12 22.1 21 19.5")


def palm():
    p0, p1, p2, p3 = (5.5, 20.5), (8, 15), (11.5, 9), (19.5, 4.5)
    out = line("M%sC%s %s %s" % (pt(*p0), pt(*p1), pt(*p2), pt(*p3)))
    for t, L in zip([0.25, 0.42, 0.58, 0.73], [6.4, 6.0, 5.2, 4.0]):
        x, y = bez(p0, p1, p2, p3, t)
        dx, dy = bezt(p0, p1, p2, p3, t)
        n = math.hypot(dx, dy)
        dx, dy = dx / n, dy / n
        for sgn in (1, -1):
            a = math.radians(62 * sgn)
            c, s = math.cos(a), math.sin(a)
            vx, vy = dx * c - dy * s, dx * s + dy * c
            ex, ey = x + L * vx, y + L * vy
            out += line("M%sQ%s %s" % (pt(x, y), pt(x + L * 0.55 * vx + 0.2 * dx * L, y + L * 0.55 * vy + 0.2 * dy * L), pt(ex - 0.15 * L * dx, ey - 0.15 * L * dy)))
    return out


def zig_ellipse(cx, cy, rxi, rxo, ryi, ryo, n):
    pts = []
    for k in range(n):
        a, b = k * 360 / n, (k + 0.5) * 360 / n
        pts.append((cx + rxi * math.sin(math.radians(a)), cy - ryi * math.cos(math.radians(a))))
        pts.append((cx + rxo * math.sin(math.radians(b)), cy - ryo * math.cos(math.radians(b))))
    return poly(pts)


def crown(cx, cy, rx, ry, n, spike):
    """A ring seen from the front with n short thorns up its far edge (Good Friday)."""
    out = '<ellipse cx="%s" cy="%s" rx="%s" ry="%s"/>' % (f(cx), f(cy), f(rx), f(ry))
    for k in range(n):
        t = -0.78 + k * (1.56 / (n - 1))
        x = cx + rx * t
        y = cy - ry * math.sqrt(max(0, 1 - t * t))
        out += line("M%sL%s" % (pt(x, y), pt(x + t * 1.4, y - spike)))
    return out


def easter():
    out = line("M2.5 20.5H21.5")
    out += line("M8.5 20.5V16C8.5 12.6 11 10.8 14 10.8C17 10.8 19.5 12.6 19.5 16V20.5")
    out += line("M12 20.5V17.6A2 2 0 0 1 16 17.6V20.5")
    out += '<circle cx="5.1" cy="16.9" r="3"/>'
    out += solid("M11.4 10.8A2.6 2.6 0 0 1 16.6 10.8Z")
    for a in (-62, -31, 0, 31, 62):
        x1, y1 = polar(14, 10.8, a, 4.5)
        x2, y2 = polar(14, 10.8, a, 6.6)
        out += line("M%sL%s" % (pt(x1, y1), pt(x2, y2)))
    return out


def grain(cx, cy, ang, L=5.6, W=3.3):
    a = math.radians(ang)
    ux, uy = math.sin(a), -math.cos(a)
    vx, vy = -uy, ux
    tip, tail = (cx + ux * L / 2, cy + uy * L / 2), (cx - ux * L / 2, cy - uy * L / 2)
    c1 = (cx + vx * W * 0.75 + ux * L * 0.1, cy + vy * W * 0.75 + uy * L * 0.1)
    c2 = (cx - vx * W * 0.75 + ux * L * 0.1, cy - vy * W * 0.75 + uy * L * 0.1)
    return solid("M%sQ%s %sQ%s %sZ" % (pt(*tail), pt(*c1), pt(*tip), pt(*c2), pt(*tail)))


def wheat():
    out = line("M12 21.5V13.5")
    for y in (14.4, 11.2, 8.0):
        out += grain(8.8, y, -30) + grain(15.2, y, 30)
    return out + grain(12, 4.9, 0, L=6.6, W=3.4)


def star8():
    pts = [(0, 10.2), (22.5, 3.4), (45, 6.6), (67.5, 3.4), (90, 8.8), (112.5, 3.4), (135, 6.6), (157.5, 3.4), (180, 8.8), (202.5, 3.4), (225, 6.6), (247.5, 3.4), (270, 8.8), (292.5, 3.4), (315, 6.6), (337.5, 3.4)]
    return poly([polar(12, 12, a, r) for a, r in pts])


DOVE_BODY = "M21.6 11.4L19.3 11.9C19.3 9.6 17.8 8.6 16.2 9.4C14.8 10.1 14.7 11.8 13.2 13C11.4 14.4 8.2 15 3.2 16.8C6.4 17.2 9.4 18.2 11.8 18.4C15.2 18.6 18.4 17 19.6 14.4C20.2 13.2 20.7 12.2 21.6 11.4Z"
DOVE_WING = "M13.4 12.4C12.6 8.6 9.8 5.6 5 4.4C5.4 8.6 7.8 12.2 12.4 14.2"
BOOK_EPIPHANY = "M12 14.2C9.9 12.7 6.5 12.4 3.2 13.4V20.4C6.5 19.4 9.9 19.7 12 21.2C14.1 19.7 17.5 19.4 20.8 20.4V13.4C17.5 12.4 14.1 12.7 12 14.2ZM12 14.2V21.2"
BOOK_BIBLE = "M12 7.5C9.5 5.6 5.5 5.2 2.6 6.2V17.6C5.5 16.6 9.5 17 12 19C14.5 17 18.5 16.6 21.4 17.6V6.2C18.5 5.2 14.5 5.6 12 7.5ZM12 7.5V19"
BIBLE_LINES = "M5.6 10.2C6.8 10 8.2 10.2 9.2 10.7M5.6 13.4C6.8 13.2 8.2 13.4 9.2 13.9M14.8 10.7C15.8 10.2 17.2 10 18.4 10.2M14.8 13.9C15.8 13.4 17.2 13.2 18.4 13.4"
CLOUD = "M5.28 21H18.06A3.41 3.41 0 0 0 18.25 13.9A5.11 5.11 0 0 0 8.58 13.5A3.6 3.6 0 0 0 5.28 21Z"


def make():
    """name: (title, season code or None, body)"""
    g = {}
    g["advent"] = ("Advent: one candle", "advent", wrap(candle(8.7, 10.5, 6.6, 10.5) + solid(teardrop(12, 2.5, 4.4, 6.4))))
    for n in range(1, 5):
        g["advent-%d" % n] = ("Advent, Sunday %d: %d of 4 candles lit" % (n, n), "advent", wrap(advent_row(n)))
    g["christmas"] = ("Christmas: the star", "christmas", wrap(solid_round(star8())))
    g["epiphany"] = ("Epiphany: light over an open book", "epiphany", wrap(solid_round(sparkle(12, 5.4, 3.9, 3.9, 0.9)) + line(BOOK_EPIPHANY)))
    g["lent"] = ("Lent: the cross", "lent", wrap(line("M10 3.5H14V8H19V12H14V20.5H10V12H5V8H10Z")))
    g["palm"] = ("Palm Sunday: a palm branch", "holy-week", wrap(palm()))
    g["good-friday"] = ("Good Friday: the cross and the crown of thorns", "good-friday", wrap(line("M12 2.5V14M7.5 6.8H16.5") + crown(12, 18.4, 8.2, 2.4, 5, 3.2)))
    g["easter"] = ("Easter: the open tomb and the sunrise", "easter", wrap(easter()))
    g["ascension"] = ("Ascension: light rising from a cloud", "ascension", wrap(solid(sparkle(12, 7.0, 3.1, 5.4, 0.8)) + line(CLOUD)))
    g["pentecost"] = ("Pentecost: the dove and the flame", "pentecost", wrap(line(shift_path(DOVE_BODY, -0.4, 1.6)) + line(shift_path(DOVE_WING, -0.4, 1.6)) + solid(flame(18.2, 2.0, 4.6, 6.6))))
    g["reformation"] = ("Reformation: the open Bible", "reformation", wrap(line(BOOK_BIBLE) + line(BIBLE_LINES)))
    g["harvest"] = ("Harvest: an ear of wheat", None, wrap(wheat()))
    g["new-year"] = ("New Year: a spark", None, wrap(solid(sparkle(12, 12, 9.2, 9.2, 1.5))))
    return g


ORDER = ["advent", "advent-1", "advent-2", "advent-3", "advent-4", "christmas", "epiphany", "lent", "palm", "good-friday", "easter", "ascension", "pentecost", "reformation", "harvest", "new-year"]

if __name__ == "__main__":
    for k in ORDER:
        print(k, len(make()[k][2]))
