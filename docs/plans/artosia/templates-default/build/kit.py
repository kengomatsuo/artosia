"""Layout helpers the four candidates share: the name grid, money rows, verse and title wrapping, the QR placeholder.

Each helper only measures and positions; a candidate's own file decides every look (bands, bars, pills, rules, colours).
"""
from engine import Sheet, Split, balanced, digits, initials, ladder, largest, wrap, W, H


def groups(rows):
    """Consecutive rows of one day: [(first_index, [rows])]."""
    out = []
    for i, r in enumerate(rows):
        if out and out[-1][1][-1]["date"] == r["date"]:
            out[-1][1].append(r)
        else:
            out.append((i, [r]))
    return out


def name_grid(sh, names, cols, top, bottom, x0, x2, gap, name_role, hl_role, size_hi, size_lo, ratio, day_cells, name_gap, pad=0.0):
    """Place names column by column. day_cells(size) -> (width of the day cell). Returns cells and the size.

    Each cell: dict(i, col, row, cx, cy, text, shortened). The size is the largest in [size_lo, size_hi] at which the
    names hold on the ladder in their column; Split when not even size_lo does (the card continues on a second slide).
    """
    n = len(names)
    per = -(-n // cols)
    colw = (x2 - x0 - gap * (cols - 1)) / cols
    avail = bottom - top
    hi = min(size_hi, int(avail / per / ratio))

    def holds(z):
        dw = day_cells(z)
        nw = colw - dw - name_gap - pad
        return all(ladder(sh, hl_role if nm["hl"] else name_role, nm["name"], nw, z) for nm in names)

    z = largest(hi, size_lo, holds)
    if z is None:
        raise Split(f"{n} names do not hold in {cols} columns at {size_lo}px")
    dw = day_cells(z)
    nw = colw - dw - name_gap - pad
    pitch = min(avail / per, ratio * 1.35 * z)
    cells = []
    for i, nm in enumerate(names):
        c, r = divmod(i, per)
        role = hl_role if nm["hl"] else name_role
        text = ladder(sh, role, nm["name"], nw, z)
        cells.append({"i": i, "col": c, "row": r, "cx": x0 + c * (colw + gap), "cy": top + r * pitch + pitch / 2, "text": text, "short": text != nm["name"]})
    sh.fit.update({"size": z, "cols": cols, "per": per, "pitch": round(pitch, 2), "colw": round(colw, 2), "day_w": round(dw, 2), "name_w": round(nw, 2),
                   "widest_px": round(max(sh.measure(hl_role if names[c["i"]]["hl"] else name_role, c["text"], z) for c in cells), 1),
                   "shortened": sum(c["short"] for c in cells)})
    return cells, z, pitch, colw


def hl_runs(names, cells, per):
    """Contiguous highlighted rows per column: [(col, first_row, last_row)]."""
    runs = []
    for c in range(-(-len(names) // per)):
        rows = [cl["row"] for cl in cells if cl["col"] == c and names[cl["i"]]["hl"]]
        if rows:
            runs.append((c, min(rows), max(rows)))
    return runs


def money_cols(sh, funds, size, label_role="fund", amount_role="amount"):
    """Width of the widest amount in digits, tabular, so the amount column is measured before the labels."""
    return max(sh.measure(amount_role, digits(f["rupiah"]), size) for f in funds)


def lines_for(sh, role, text, width, size):
    """text in balanced lines of at most width."""
    face = sh.face(role)
    space = face.adv(" ", size)
    words = text.split()
    return [" ".join(l) for l in balanced(words, width, space, lambda s: face.adv(s, size, sh.ls_px(role, size)))]


def verse_lines(sh, verses, width, size, scale, role="verse"):
    """Greedy-balanced lines of tokens ('n', number) and ('w', word); a number is raised and set in the same line."""
    face = sh.face(role)
    space = face.adv(" ", size)
    toks = []
    for n, t in verses:
        toks.append(("n", str(n)))
        toks += [("w", w) for w in t.split()]
    num_w = lambda s: face.adv(s, size * scale) + 0.08 * size
    wd = lambda t: num_w(t[1]) if t[0] == "n" else face.adv(t[1], size)

    def greedy(wid):
        lines, cur, used = [], [], 0.0
        for t in toks:
            w = wd(t)
            glue = 0 if (not cur or t[0] == "n") else space
            if cur and t[0] != "n" and used + glue + w > wid:
                lines.append(cur)
                cur, used = [t], w
            elif cur and t[0] == "n" and used + w > wid:
                lines.append(cur)
                cur, used = [t], w
            else:
                cur.append(t)
                used += glue + w
        if cur:
            lines.append(cur)
        return lines

    count = len(greedy(width))
    lo, hi = width * 0.5, width
    for _ in range(18):
        mid = (lo + hi) / 2
        if len(greedy(mid)) <= count:
            hi = mid
        else:
            lo = mid
    return greedy(hi)


def draw_verse(sh, lines, x, top, size, lh, role, num_role, num_fill, scale, rise, id=""):
    """Draw token lines; numbers raised and smaller. Returns the bottom."""
    face = sh.face(role)
    space = face.adv(" ", size)
    cap = face.cap * size
    for i, line in enumerate(lines):
        base = top + (i + 0.5) * lh * size + cap / 2
        cx, run = x, []

        def flush():
            nonlocal cx, run
            if run:
                it = sh.text(role, " ".join(run), cx, base, size=size, id=f"{id}.l{i}")
                cx += it["w"] + space
                run = []

        for t in line:
            if t[0] == "n":
                flush()
                it = sh.text(num_role, t[1], cx, base - rise * size, size=max(32, round(size * scale)), fill=num_fill, id=f"{id}.n{t[1]}")
                cx += it["w"] + 0.08 * size
            else:
                run.append(t[1])
        flush()
    return top + len(lines) * lh * size


def title_lines(sh, role, text, width, max_lines, hi, lo, upper=False):
    """Largest size at which the title wraps to at most max_lines lines of width."""
    words = (text.upper() if upper else text).split()

    def lines_at(z):
        face = sh.face(role)
        space = face.adv(" ", z)
        return balanced(words, width, space, lambda s: face.adv(s, z, sh.ls_px(role, z)))

    z = largest(hi, lo, lambda z: len(lines_at(z)) <= max_lines)
    if z is None:
        raise Split("title")
    return z, [" ".join(l) for l in lines_at(z)]


def qr(sh, x, y, s, ink, paper, ground, pad, id="qr"):
    """A QR code placeholder: a quiet zone card in the light colour (a code must be dark on light, so a dark ground gets a light card),
    three finder squares and a module field. Wireframe only; the real code is the church's own picture."""
    sh.rect(x, y, s, s, fill=paper, id=f"{id}.card")
    inner = s - 2 * pad
    m = inner / 21
    sh.rect(x + pad, y + pad, inner, inner, fill=None, id=f"{id}.zone")
    for fx, fy in ((0, 0), (14, 0), (0, 14)):
        sh.rect(x + pad + fx * m, y + pad + fy * m, 7 * m, 7 * m, fill=ink, id=f"{id}.f{fx}{fy}")
        sh.rect(x + pad + (fx + 1) * m, y + pad + (fy + 1) * m, 5 * m, 5 * m, fill=paper, id=f"{id}.fi{fx}{fy}")
        sh.rect(x + pad + (fx + 2) * m, y + pad + (fy + 2) * m, 3 * m, 3 * m, fill=ink, id=f"{id}.fc{fx}{fy}")
    seed = 7
    for r in range(21):
        for c in range(21):
            if (r < 8 and c < 8) or (r < 8 and c > 12) or (r > 12 and c < 8):
                continue
            seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
            if seed % 20 < 7:
                sh.rect(x + pad + c * m, y + pad + r * m, m, m, fill=ink, id=f"{id}.m{r}.{c}")


def fit_events(sh, rows, role, hl_role, width, hi, lo, fits):
    """Fit.rs fit_cells: the largest size at which every event is on one line and fits(z, 0) holds; failing that,
    the largest where each is on at most two balanced lines and fits(z, extra) holds, extra being the second line's height.
    Returns (size, lines per row, extra) or None."""
    role_of = lambda r: hl_role if r["special"] else role
    wid = width if callable(width) else (lambda r: width)
    one = largest(hi, lo, lambda z: fits(z, 0.0) and all(sh.measure(role_of(r), r["event"], z) <= wid(r) for r in rows))
    if one:
        return one, [[r["event"]] for r in rows], 0.0

    def two(z):
        out = []
        for r in rows:
            if sh.measure(role_of(r), r["event"], z) <= wid(r):
                out.append([r["event"]])
                continue
            ls = lines_for(sh, role_of(r), r["event"], wid(r), z)
            if len(ls) > 2 or any(sh.measure(role_of(r), l, z) > wid(r) for l in ls):
                return None
            out.append(ls)
        extra = 1.12 * z if any(len(l) > 1 for l in out) else 0.0
        return out if fits(z, extra) else None

    z = largest(hi, lo, lambda z: two(z) is not None)
    if z is None:
        return None
    lines = two(z)
    return z, lines, (1.12 * z if any(len(l) > 1 for l in lines) else 0.0)
