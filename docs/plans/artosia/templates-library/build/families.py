"""The birthday (S02) and offering (S03) wireframes of the eight families, as resolved scenes.

Every constant a layout needs is a token in spec.json (families.<id>.tokens); the rest is arithmetic on the sample data.
A function is the layout rule a builder ports: it names, in order, what is drawn and where.
"""
import datetime

from lib import Scene, day_of, ladder, rupiah, balanced, r2

LINE = 1.12  # the engine's height of one more line of text, as a share of its size


# ------------------------------------------------------------------ shared pieces

def names_of(spec):
    d = spec["data"]["birthdays"]
    return [dict(d=n[0], m=n[1], name=n[2], hl=bool(n[3])) for n in d["names"]]


def funds_of(spec):
    return [dict(label=f[0], rp=f[1]) for f in spec["data"]["offerings"]["funds"]]


def fit_names(sc, names, role, role_hl, width, floor):
    """The size and the text of every name: the largest size, up to the role's own, that shortens no more names than the floor size does."""
    hi = int(sc.role(role)[1])

    def at(size):
        out, cut = [], 0
        for n in names:
            got = ladder(n["name"], lambda t: sc.w(role_hl if n["hl"] else role, t, size) <= width * 0.985)
            if got is None:
                return None
            cut += got[1] > 0
            out.append(got[0])
        return cut, out

    base = at(int(floor))
    size, best = int(floor), base
    for z in range(int(floor) + 1, hi + 1):
        got = at(z)
        if got and base and got[0] <= base[0]:
            size, best = z, got
    cut, shown = best
    widest = max(sc.w(role_hl if n["hl"] else role, n["name"], size) for n in names)
    sc.fits.update(name_size=size, name_cell=r2(width), names_shortened=cut, longest_name_px=r2(widest), names=len(names))
    return size, shown


def fit_label(sc, role, text, avail, lo, room):
    """A label on one line at its role's size, else the largest size >= lo where it fits on one line, else two balanced lines in `room` height."""
    key, hi, ls, caps, _ = sc.role(role)
    stem = sc.stem(key)
    for size in range(int(hi), int(lo) - 1, -1):
        if sc.fonts.width(stem, text, size, ls) <= avail * 0.985:
            return [text], size
    size = int(hi)
    while size > lo and 2 * LINE * size > room:
        size -= 1
    return balanced(sc.fonts, stem, text, size, avail, ls), size


def day_width(sc, cp, names, floor, role="bday_day"):
    """The date column: the widest 'dd Mon' as set, never less than the family's floor token."""
    return max(floor, max(sc.w(role, day_of(cp, n)) for n in names))


def money(sc, xr, cy, n, role="amount", rp="rp", size=None, fill=None, rp_fill=None, gap=12):
    """Rupiah right-aligned at xr: the digits, and Rp in its own place before them. Returns the whole width."""
    w = sc.text(role, rupiah(n), xr, cy, anchor="end", size=size, fill=fill)
    rw = sc.text(rp, sc.fam["copy_rp"], xr - w - gap, cy, anchor="end", fill=rp_fill)
    return w + gap + rw


def hl_runs(names, per):
    """Per column, the first row and the length of each run of highlighted rows: [(col, first_row, count)]."""
    runs = []
    for c in range(0, (len(names) + per - 1) // per):
        col = names[c * per:(c + 1) * per]
        i = 0
        while i < len(col):
            if col[i]["hl"]:
                j = i
                while j + 1 < len(col) and col[j + 1]["hl"]:
                    j += 1
                runs.append((c, i, j - i + 1))
                i = j + 1
            else:
                i += 1
    return runs


def month_cols(names, per):
    """Names per column with a month heading before the first name of each month: [("h", month) | ("n", index)]."""
    cols = []
    for c in range((len(names) + per - 1) // per):
        items, last = [], None
        for i in range(c * per, min(len(names), (c + 1) * per)):
            if names[i]["m"] != last:
                items.append(("h", names[i]["m"]))
                last = names[i]["m"]
            items.append(("n", i))
        cols.append(items)
    return cols


def item_runs(names, cols):
    """Runs of highlighted names over the heading-and-name items of each column: [(col, first_item, count)]."""
    runs = []
    for c, items in enumerate(cols):
        j = 0
        while j < len(items):
            if items[j][0] == "n" and names[items[j][1]]["hl"]:
                k = j
                while k + 1 < len(items) and items[k + 1][0] == "n" and names[items[k + 1][1]]["hl"]:
                    k += 1
                runs.append((c, j, k - j + 1))
                j = k + 1
            else:
                j += 1
    return runs


def alt_shortened(sc, names, role, role_hl, cell, size, saved):
    """How many names would take initials at `size` if each row also wrote its month, which costs `saved` more px of date column."""
    cut = 0
    for n in names:
        got = ladder(n["name"], lambda t: sc.w(role_hl if n["hl"] else role, t, size) <= (cell - saved) * 0.985)
        cut += (got is None) or got[1] > 0
    return cut


def kicker_text(spec, sc):
    d = spec["data"]
    return (d["copy"]["warta"] + d["copy"]["sep"] + d["church"]).upper()


def new(spec, fid, fonts):
    fam = spec["families"][fid]
    fam = dict(fam, copy_rp=spec["data"]["copy"]["rp"])
    return Scene(fam, fonts, spec["sheet"]["w"], spec["sheet"]["h"]), fam["tokens"], spec["data"]


def pct(rp, total):
    return f"{rp / total * 100:.1f}".replace(".", ",") + "%"


# ------------------------------------------------------------------ 1 diagonal

def diagonal_header(sc, T, kicker, title, sub):
    W = sc.W
    sc.poly([(0, 0), (T["hdr_w"], 0), (T["hdr_w"] - T["cut"], T["hdr_h"]), (0, T["hdr_h"])], "brand_dk")
    x0 = T["hdr_w"] + T["gap"]
    sc.poly([(x0, 0), (x0 + T["stripe_w"], 0), (x0 + T["stripe_w"] - T["cut"], T["hdr_h"]), (x0 - T["cut"], T["hdr_h"])], "brand")
    sc.text("kicker", kicker, T["mx"], cy=T["kicker_cy"], fill="sky")
    sc.text("title", title, T["mx"], cy=T["title_cy"])
    sc.text("sub", sub, T["mx"], cy=T["sub_cy"])


def diagonal_S02(spec, fonts):
    sc, T, D = new(spec, "diagonal", fonts)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    diagonal_header(sc, T, kicker_text(spec, sc), cp["birthdays"], B["month_label"])
    # the range, with Jatibaru's arrow, right-aligned on the title's line
    xr = sc.W - T["mx"]
    sc.text("kicker", cp["week_this"], xr, cy=T["chip_k_cy"], anchor="end", fill="mute")
    w = sc.text("tag", B["week_label"], xr, cy=T["chip_cy"], anchor="end", size=T["chip_size"], font="h8", fill="brand")
    ax = xr - w - T["arrow_gap"]
    a = T["arrow"]
    sc.poly([(ax - a[0], T["chip_cy"] - a[1]), (ax - a[0] + a[2], T["chip_cy"] - a[1]), (ax - a[0] + a[2], T["chip_cy"] - a[3]), (ax, T["chip_cy"]),
             (ax - a[0] + a[2], T["chip_cy"] + a[3]), (ax - a[0] + a[2], T["chip_cy"] + a[1]), (ax - a[0], T["chip_cy"] + a[1])], "brand")
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    tw = day_width(sc, cp, names, T["tile_w"] - 2 * T["tile_pad"]) + 2 * T["tile_pad"]
    nx = tw + T["tile_gap"]
    size, shown = fit_names(sc, names, "name", "name_hl", cw - nx, T["name_floor"])
    sc.fits["day_col"] = r2(tw)
    pitch = (T["body_bot"] - T["body_top"]) / per
    sc.fits["pitch"] = r2(pitch)
    for c, i0, n in hl_runs(names, per):
        sc.rect(T["mx"] + c * (cw + g) - T["band_pad"], T["body_top"] + i0 * pitch + 3, cw + 2 * T["band_pad"], n * pitch - 6, "tint", rx=T["tile_rx"])
    for i, n in enumerate(names):
        c, r = divmod(i, per)
        x, cy = T["mx"] + c * (cw + g), T["body_top"] + (r + 0.5) * pitch
        sc.rect(x, cy - T["tile_h"] / 2, tw, T["tile_h"], "brand" if n["hl"] else "panel", rx=T["tile_rx"], stroke=None if n["hl"] else "line", sw=None if n["hl"] else T["tile_sw"])
        sc.text("bday_day", day_of(cp, n), x + tw / 2, cy=cy, anchor="middle", fill="on_brand" if n["hl"] else None)
        sc.text("name_hl" if n["hl"] else "name", shown[i], x + nx, cy=cy, size=size)
    return sc


def diagonal_S03(spec, fonts):
    sc, T, D = new(spec, "diagonal", fonts)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    diagonal_header(sc, T, kicker_text(spec, sc), O["title"], O["date_long"])
    n = len(funds)
    pitch = (T["rows_bot"] - T["rows_top"]) / n
    sc.fits["pitch"] = r2(pitch)
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    lab_x = T["mx"] + T["no_w"] + T["no_gap"]
    avail = sc.W - T["mx"] - amount_w - T["lab_gap"] - lab_x
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["rows_top"] + (i + 0.5) * pitch
        sc.rect(T["mx"], cy - T["no_w"] / 2, T["no_w"], T["no_w"], "brand", rx=T["tile_rx"])
        sc.text("bday_day", str(i + 1), T["mx"] + T["no_w"] / 2, cy=cy, anchor="middle", fill="on_brand")
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 12)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, lab_x, cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, sc.W - T["mx"], cy, f["rp"])
        if i + 1 < n:
            sc.line(T["mx"], cy + pitch / 2, sc.W - T["mx"], cy + pitch / 2, "line", T["rule_w"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    tp, off = T["total_top"], T["cut"] * (sc.H - T["total_top"]) / T["hdr_h"]
    sc.poly([(off, tp), (sc.W, tp), (sc.W, sc.H), (0, sc.H)], "brand")
    sc.text("total_label", cp["total_gifts"], off + T["total_pad"], cy=(tp + sc.H) / 2)
    w = sc.text("total", rupiah(total), sc.W - T["mx"], cy=(tp + sc.H) / 2, anchor="end")
    sc.text("total", cp["rp"], sc.W - T["mx"] - w - 18, cy=(tp + sc.H) / 2, anchor="end", size=T["total_rp"])
    sc.fits["total_px"] = r2(w)
    return sc


# ------------------------------------------------------------------ 2 liturgi

def liturgi_frame(sc, T, title, sub, spec, chip=None):
    W, H, cx = sc.W, sc.H, sc.W / 2
    sc.rect(0, 0, W, H, "ground2")
    sc.rect(T["frame"], T["frame"], W - 2 * T["frame"], H - 2 * T["frame"], "ground", stroke="gold", sw=T["frame_sw"])
    sc.rect(T["frame"] + T["frame_gap"], T["frame"] + T["frame_gap"], W - 2 * (T["frame"] + T["frame_gap"]), H - 2 * (T["frame"] + T["frame_gap"]), stroke="gold", sw=T["frame_sw2"], op=T["frame_op2"])
    for x in (T["frame"] + T["frame_gap"], W - T["frame"] - T["frame_gap"]):
        for y in (T["frame"] + T["frame_gap"], H - T["frame"] - T["frame_gap"]):
            sc.rect(x - T["dia"] / 2, y - T["dia"] / 2, T["dia"], T["dia"], "gold", rot=45)
    cy = T["cross_cy"]
    sc.rect(cx - T["cross_w"] / 2, cy - T["cross_h"] / 2, T["cross_w"], T["cross_h"], "gold")
    sc.rect(cx - T["cross_arm"] / 2, cy - T["cross_arm_h"] / 2 - T["cross_up"], T["cross_arm"], T["cross_arm_h"], "gold")
    sc.line(T["rule_x"], cy, cx - T["rule_gap"], cy, "gold", T["rule_w"])
    sc.line(cx + T["rule_gap"], cy, W - T["rule_x"], cy, "gold", T["rule_w"])
    sc.text("kicker", kicker_text(spec, sc), cx, cy=T["kicker_cy"], anchor="middle")
    sc.text("title", title, cx, cy=T["title_cy"], anchor="middle")


def liturgi_S02(spec, fonts):
    sc, T, D = new(spec, "liturgi", fonts)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    liturgi_frame(sc, T, cp["birthdays"], B["month_label"], spec)
    sc.text("sub", B["month_label"], T["pad"], cy=T["sub_cy"])
    xr = sc.W - T["pad"]
    tw = sc.w("tag", cp["week_this"] + "  " + B["week_label"])
    sc.rect(xr - tw - 2 * T["chip_pad"], T["sub_cy"] - T["chip_h"] / 2, tw + 2 * T["chip_pad"], T["chip_h"], stroke="gold", sw=T["chip_sw"], rx=T["chip_h"] / 2)
    sc.text("tag", cp["week_this"] + "  " + B["week_label"], xr - T["chip_pad"], cy=T["sub_cy"], anchor="end")
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["pad"] - 2 * g) / 3
    dw = day_width(sc, cp, names, T["day_w"])
    size, shown = fit_names(sc, names, "name", "name_hl", cw - dw - T["day_gap"] - T["row_pad"], T["name_floor"])
    sc.fits["day_col"] = r2(dw)
    pitch = (T["body_bot"] - T["body_top"]) / per
    sc.fits["pitch"] = r2(pitch)
    for c in (1, 2):
        x = T["pad"] + c * (cw + g) - g / 2
        sc.line(x, T["body_top"], x, T["body_bot"], "gold", T["vrule_w"], op=T["vrule_op"])
    for c, i0, n in hl_runs(names, per):
        sc.rect(T["pad"] + c * (cw + g) - T["band_over"], T["body_top"] + i0 * pitch + 3, cw + 2 * T["band_over"], n * pitch - 6, "cream", rx=T["band_rx"])
    for i, n in enumerate(names):
        c, r = divmod(i, per)
        x, cy = T["pad"] + c * (cw + g), T["body_top"] + (r + 0.5) * pitch
        sc.text("bday_day", day_of(cp, n), x + T["row_pad"], cy=cy, fill="ink" if n["hl"] else None)
        sc.text("name_hl" if n["hl"] else "name", shown[i], x + T["row_pad"] + dw + T["day_gap"], cy=cy, size=size)
    return sc


def liturgi_S03(spec, fonts):
    sc, T, D = new(spec, "liturgi", fonts)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    liturgi_frame(sc, T, O["title"], O["date_long"], spec)
    sc.text("sub", O["date_long"], sc.W / 2, cy=T["sub_cy"], anchor="middle")
    n = len(funds)
    pitch = (T["rows_bot"] - T["rows_top"]) / n
    sc.fits["pitch"] = r2(pitch)
    xr = sc.W - T["pad"]
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - amount_w - T["lab_gap"] - T["pad"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["rows_top"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 12)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, T["pad"], cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr, cy, f["rp"])
        if i + 1 < n:
            sc.line(T["pad"], cy + pitch / 2, xr, cy + pitch / 2, "gold", T["rule_w"], op=T["rule_op"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    sc.rect(T["pad"], T["total_top"], xr - T["pad"], T["total_h"], "cream", rx=T["band_rx"])
    cy = T["total_top"] + T["total_h"] / 2
    sc.text("total_label", cp["total_gifts"], T["pad"] + T["total_pad"], cy=cy)
    w = sc.text("total", rupiah(total), xr - T["total_pad"], cy=cy, anchor="end")
    sc.text("total", cp["rp"], xr - T["total_pad"] - w - 16, cy=cy, anchor="end", size=T["total_rp"])
    sc.fits["total_px"] = r2(w)
    return sc


# ------------------------------------------------------------------ 3 kaca (photo-led)

def kaca_ground(sc, T, L):
    sc.photo(0, 0, sc.W, sc.H, "dusk_a", "dusk_b", T["blobs"])
    sc.rect(0, 0, sc.W, sc.H, "scrim", op=L["scrim_photo"]["op"])


def kaca_S02(spec, fonts):
    sc, T, D = new(spec, "kaca", fonts)
    B, cp, L = D["birthdays"], D["copy"], spec["families"]["kaca"]["layers"]
    names = names_of(spec)
    kaca_ground(sc, T, L)
    sc.text("kicker", kicker_text(spec, sc), T["mx"], cy=T["kicker_cy"], op=T["kicker_op"])
    sc.text("title", cp["birthdays"], T["mx"] - T["title_dx"], base=T["title_base"])
    xr = sc.W - T["mx"]
    sc.text("sub", B["month_label"], xr, cy=T["sub_cy"], anchor="end")
    label = cp["week_this"] + " · " + B["week_label"]
    tw = sc.w("tag", label, font="j6")
    sc.rect(xr - tw - 2 * T["pill_pad"], T["pill_cy"] - T["pill_h"] / 2, tw + 2 * T["pill_pad"], T["pill_h"], "plate", op=L["plate_photo"]["op"], rx=T["pill_h"] / 2)
    sc.text("tag", label, xr - T["pill_pad"], cy=T["pill_cy"], anchor="end", fill="accent", font="j6")
    sc.rect(T["plate_x"], T["plate_y"], sc.W - 2 * T["plate_x"], T["plate_h"], "plate", op=L["plate_photo"]["op"], rx=T["plate_rx"])
    per, g = T["per"], T["gutter"]
    inner = sc.W - 2 * T["plate_x"] - 2 * T["plate_pad"]
    cw = (inner - 2 * g) / 3
    dw = day_width(sc, cp, names, T["day_w"])
    nx = T["day_in"] + dw + T["day_gap"]
    size, shown = fit_names(sc, names, "name", "name_hl", cw - nx, T["name_floor"])
    sc.fits["day_col"] = r2(dw)
    pitch = (T["plate_h"] - 2 * T["plate_pad_v"]) / per
    sc.fits["pitch"] = r2(pitch)
    y0 = T["plate_y"] + T["plate_pad_v"]
    for c, i0, n in hl_runs(names, per):
        sc.rect(T["plate_x"] + T["plate_pad"] + c * (cw + g) - T["band_over"], y0 + i0 * pitch + 3, cw + 2 * T["band_over"], n * pitch - 6, "accent_tint", rx=T["band_rx"])
        sc.rect(T["plate_x"] + T["plate_pad"] + c * (cw + g) - T["band_over"], y0 + i0 * pitch + 3, T["bar_w"], n * pitch - 6, "accent", rx=T["bar_w"] / 2)
    for i, n in enumerate(names):
        c, r = divmod(i, per)
        x, cy = T["plate_x"] + T["plate_pad"] + c * (cw + g), y0 + (r + 0.5) * pitch
        sc.text("bday_day", day_of(cp, n), x + T["day_in"], cy=cy)
        sc.text("name_hl" if n["hl"] else "name", shown[i], x + nx, cy=cy, size=size)
    return sc


def kaca_S03(spec, fonts):
    sc, T, D = new(spec, "kaca", fonts)
    O, cp, L = D["offerings"], D["copy"], spec["families"]["kaca"]["layers"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    kaca_ground(sc, T, L)
    sc.text("kicker", kicker_text(spec, sc), T["mx"], cy=T["kicker_cy"], op=T["kicker_op"])
    sc.text("title", O["title"], T["mx"] - T["title_dx"], base=T["title_base"])
    sc.text("sub", O["date_long"], sc.W - T["mx"], cy=T["sub_cy"], anchor="end")
    sc.rect(T["plate_x"], T["plate_y"], T["plate_w"], T["plate_h"], "plate", op=L["plate_photo"]["op"], rx=T["plate_rx"])
    n = len(funds)
    pitch = (T["plate_h"] - 2 * T["plate_pad_v"]) / n
    sc.fits["pitch"] = r2(pitch)
    xl, xr = T["plate_x"] + T["plate_pad"], T["plate_x"] + T["plate_w"] - T["plate_pad"]
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - xl - amount_w - T["lab_gap"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["plate_y"] + T["plate_pad_v"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 12)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, xl, cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr, cy, f["rp"])
        if i + 1 < n:
            sc.line(xl, cy + pitch / 2, xr, cy + pitch / 2, "mute", T["rule_w"], op=T["rule_op"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    sc.rect(T["tot_x"], T["tot_y"], sc.W - T["mx"] - T["tot_x"], T["tot_h"], "accent", rx=T["plate_rx"])
    cx = T["tot_x"] + T["tot_pad"]
    sc.text("total_label", cp["total_gifts"], cx, cy=T["tot_y"] + T["tot_label_dy"])
    sc.text("total", cp["rp"], cx, cy=T["tot_y"] + T["tot_rp_dy"], size=T["total_rp"], font="j5")
    size = T["total_fit"]
    inner = sc.W - T["mx"] - cx - T["tot_pad"]
    while sc.w("total", rupiah(total), size) > inner and size > T["total_min"]:
        size -= 1
    w = sc.text("total", rupiah(total), cx, cy=T["tot_y"] + T["tot_fig_dy"], size=size)
    sc.fits.update(total_px=r2(w), total_size=size)
    return sc


# ------------------------------------------------------------------ 4 blok

def blok_band(sc, T, kicker, title, sub, right_a, right_b, band_h):
    sc.rect(0, 0, sc.W, sc.H, "deep")
    sc.rect(0, 0, sc.W, band_h, "signal")
    sc.rect(T["blue_x"], 0, sc.W - T["blue_x"], band_h, "field2")
    sc.text("kicker", kicker, T["mx"], cy=T["kicker_cy"])
    sc.text("title", title, T["mx"], cy=band_h - T["title_up"])
    sc.text("sub", sub, T["mx"], cy=band_h - T["sub_up"])
    bx = T["blue_x"] + T["blue_pad"]
    sc.text("kicker", right_a, bx, cy=T["kicker_cy"], fill="white")
    return bx


def blok_S02(spec, fonts):
    sc, T, D = new(spec, "blok", fonts)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    bx = blok_band(sc, T, kicker_text(spec, sc), cp["birthdays"], B["month_label"], cp["week_this"], None, T["band_h"])
    sc.text("sub", B["week_label"], bx, cy=T["band_h"] - T["sub_up"] - T["chip_dy"], size=T["chip_size"], fill="white", font="p7", caps=True)
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    dw = day_width(sc, cp, names, T["day_w"])
    size, shown = fit_names(sc, names, "name", "name_hl", cw - dw - T["day_gap"] - 2 * T["row_pad"], T["name_floor"])
    sc.fits["day_col"] = r2(dw)
    pitch = (T["body_bot"] - T["body_top"]) / per
    sc.fits["pitch"] = r2(pitch)
    for c, i0, n in hl_runs(names, per):
        sc.rect(T["mx"] + c * (cw + g), T["body_top"] + i0 * pitch + 2, cw, n * pitch - 4, "signal")
    for i, n in enumerate(names):
        c, r = divmod(i, per)
        x, cy = T["mx"] + c * (cw + g), T["body_top"] + (r + 0.5) * pitch
        sc.text("bday_day", day_of(cp, n), x + T["row_pad"], cy=cy, fill="deep" if n["hl"] else None)
        sc.text("name_hl" if n["hl"] else "name", shown[i], x + T["row_pad"] + dw + T["day_gap"], cy=cy, size=size)
        if r + 1 < per and not (n["hl"] and names[i + 1]["hl"]) and not n["hl"] and not names[i + 1]["hl"]:
            sc.line(x, cy + pitch / 2, x + cw, cy + pitch / 2, "mute", T["rule_w"], op=T["rule_op"])
    return sc


def blok_S03(spec, fonts):
    sc, T, D = new(spec, "blok", fonts)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    bx = blok_band(sc, T, kicker_text(spec, sc), O["title"], cp["kas"], O["day_name"], None, T["band_h3"])
    sc.text("sub", O["date_short"], bx, cy=T["band_h3"] - T["sub_up"] - T["chip_dy"], size=T["chip_size"], fill="white", font="p7", caps=True)
    n = len(funds)
    pitch = (T["rows_bot"] - T["rows_top"]) / n
    sc.fits["pitch"] = r2(pitch)
    xr = sc.W - T["mx"]
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - amount_w - T["lab_gap"] - T["mx"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["rows_top"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 12)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, T["mx"], cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr, cy, f["rp"])
        if i + 1 < n:
            sc.line(T["mx"], cy + pitch / 2, xr, cy + pitch / 2, "mute", T["rule_w"], op=T["rule_op"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    sc.rect(0, T["total_top"], sc.W, sc.H - T["total_top"], "signal")
    cy = (T["total_top"] + sc.H) / 2
    sc.text("total_label", cp["total_gifts"], T["mx"], cy=cy)
    lw = sc.w("total_label", cp["total_gifts"])
    room = xr - T["mx"] - lw - T["tot_gap"] - sc.w("rp", cp["rp"]) - 16
    size = T["total_max"]
    while sc.w("total", rupiah(total), size) > room and size > T["total_min"]:
        size -= 1
    w = sc.text("total", rupiah(total), xr, cy=cy, anchor="end", size=size)
    sc.text("total", cp["rp"], xr - w - 16, cy=cy, anchor="end", size=T["total_rp"])
    sc.fits.update(total_px=r2(w), total_size=size)
    return sc


# ------------------------------------------------------------------ 5 cahaya

def cahaya_ground(sc, T, L):
    sc.photo(0, 0, sc.W, sc.H, "g1", "g2", [])
    for g in T["glows"]:
        sc.glow(g["cx"], g["cy"], g["r"], [[0, g["c"], g["a"]], [g["mid"], g["c2"], g["mid_a"]], [1, g["c2"], 0]])


def cahaya_title(sc, T, title, sub, spec):
    sc.text("kicker", kicker_text(spec, sc), T["mx"], cy=T["kicker_cy"])
    sc.text("title", title, T["mx"], base=T["title_base"])
    sc.text("sub", sub, T["mx"], cy=T["sub_cy"])


def cahaya_pill(sc, T, label):
    xr = sc.W - T["mx"]
    tw = sc.w("tag", label)
    sc.rect(xr - tw - 2 * T["pill_pad"], T["pill_cy"] - T["pill_h"] / 2, tw + 2 * T["pill_pad"], T["pill_h"], "pill", rx=T["pill_h"] / 2)
    sc.text("tag", label, xr - T["pill_pad"], cy=T["pill_cy"], anchor="end")


def cahaya_S02(spec, fonts):
    sc, T, D = new(spec, "cahaya", fonts)
    B, cp, L = D["birthdays"], D["copy"], spec["families"]["cahaya"]["layers"]
    names = names_of(spec)
    cahaya_ground(sc, T, L)
    cahaya_title(sc, T, cp["birthdays"], B["month_label"], spec)
    cahaya_pill(sc, T, cp["week_this"] + "  " + B["week_label"])
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    dw = max(T["day_w"], max(sc.w("bday_day", f"{n['d']:02d}") for n in names))
    full = day_width(sc, cp, names, 0)
    nx = dw + T["day_gap"]
    size, shown = fit_names(sc, names, "name", "name_hl", cw - nx - T["row_pad"], T["name_floor"])
    sc.fits.update(day_col=r2(dw), shortened_if_each_row_names_its_month=alt_shortened(sc, names, "name", "name_hl", cw - nx - T["row_pad"], size, full - dw))
    cols = month_cols(names, per)
    rows = max(len(c) for c in cols)
    pitch = (T["body_bot"] - T["body_top"]) / rows
    sc.fits["pitch"] = r2(pitch)
    for c, j0, n in item_runs(names, cols):
        x = T["mx"] + c * (cw + g) - T["band_over"]
        sc.rect(x, T["body_top"] + j0 * pitch + 3, cw + 2 * T["band_over"], n * pitch - 6, "glass", op=L["glass_on_g2"]["op"], rx=T["band_rx"])
        sc.rect(x + T["bar_in"], T["body_top"] + j0 * pitch + 3 + T["bar_pad"], T["bar_w"], n * pitch - 6 - 2 * T["bar_pad"], "accent", rx=T["bar_w"] / 2)
    for c, items in enumerate(cols):
        x = T["mx"] + c * (cw + g)
        for j, (kind, v) in enumerate(items):
            cy = T["body_top"] + (j + 0.5) * pitch
            if kind == "h":
                sc.text("caption", cp["months_long"][v - 1], x + T["day_in"], cy=cy)
            else:
                n = names[v]
                sc.text("bday_day", f"{n['d']:02d}", x + T["day_in"], cy=cy)
                sc.text("name_hl" if n["hl"] else "name", shown[v], x + nx, cy=cy, size=size)
    return sc


def cahaya_S03(spec, fonts):
    sc, T, D = new(spec, "cahaya", fonts)
    O, cp, L = D["offerings"], D["copy"], spec["families"]["cahaya"]["layers"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    cahaya_ground(sc, T, L)
    cahaya_title(sc, T, O["title"], cp["kas"], spec)
    cahaya_pill(sc, T, O["date_long"])
    n = len(funds)
    pitch = (T["rows_bot"] - T["rows_top"]) / n
    sc.fits["pitch"] = r2(pitch)
    xr = T["list_r"]
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - amount_w - T["lab_gap"] - T["mx"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["rows_top"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 8)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, T["mx"], cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr, cy, f["rp"])
        if i + 1 < n:
            sc.line(T["mx"], cy + pitch / 2, xr, cy + pitch / 2, "mute", T["rule_w"], op=T["rule_op"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    xt = sc.W - T["mx"]
    sc.text("total_label", cp["total_gifts"], xt, cy=T["tot_label_cy"], anchor="end")
    size = T["total_max"]
    rw = sc.w("rp", cp["rp"], size=T["total_rp"], font="s3") + 20
    while sc.w("total", rupiah(total), size) + rw > xt - T["mx"] and size > T["total_min"]:
        size -= 1
    w = sc.text("total", rupiah(total), xt, cy=T["tot_fig_cy"], anchor="end", size=size)
    sc.text("total", cp["rp"], xt - w - 20, cy=T["tot_fig_cy"], anchor="end", size=T["total_rp"])
    sc.fits.update(total_px=r2(w), total_size=size)
    return sc


# ------------------------------------------------------------------ 6 majalah

def majalah_page(sc, T, title, sub, spec, aside=None):
    cp = spec["data"]["copy"]
    sc.rect(0, 0, sc.W, sc.H, "ground")
    sc.text("kicker", cp["warta"], T["mx"], cy=T["meta_top_cy"], fill="mute")
    sc.text("kicker", spec["data"]["church"], sc.W - T["mx"], cy=T["meta_top_cy"], anchor="end", fill="mute")
    sc.rect(T["mx"], T["rule_y"], sc.W - 2 * T["mx"], T["rule_thick"], "ink")
    sc.rect(T["mx"], T["rule_y"] + T["rule_thick"] + T["rule_gap"], sc.W - 2 * T["mx"], T["rule_thin"], "ink")
    w = sc.text("title", title, T["mx"], base=T["title_base"])
    sc.text("sub", sub, sc.W - T["mx"], base=T["title_base"], anchor="end")
    if aside:
        sc.text("tag", aside, sc.W - T["mx"], cy=T["aside_cy"], anchor="end")
    return w


def majalah_S02(spec, fonts):
    sc, T, D = new(spec, "majalah", fonts)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    majalah_page(sc, T, cp["birthdays"], B["month_label"], spec, cp["week_this"] + " " + B["week_label"])
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    dw = max(T["day_w"], max(sc.w("bday_day", f"{n['d']:02d}") for n in names))
    full = day_width(sc, cp, names, 0)
    cell = cw - dw - T["day_gap"] - T["row_pad"]
    size, shown = fit_names(sc, names, "name", "name_hl", cell, T["name_floor"])
    sc.fits.update(day_col=r2(dw), shortened_if_each_row_names_its_month=alt_shortened(sc, names, "name", "name_hl", cell, size, full - dw))
    cols = month_cols(names, per)
    rows = max(len(c) for c in cols)
    pitch = (T["body_bot"] - T["body_top"]) / rows
    sc.fits["pitch"] = r2(pitch)
    for c in (1, 2):
        x = T["mx"] + c * (cw + g) - g / 2
        sc.line(x, T["body_top"], x, T["body_bot"], "rule", T["vrule_w"])
    for c, j0, n in item_runs(names, cols):
        sc.rect(T["mx"] + c * (cw + g) - T["band_over"], T["body_top"] + j0 * pitch + 2, cw + 2 * T["band_over"], n * pitch - 4, "inv_bg")
    for c, items in enumerate(cols):
        x = T["mx"] + c * (cw + g)
        for j, (kind, v) in enumerate(items):
            cy = T["body_top"] + (j + 0.5) * pitch
            if kind == "h":
                sc.text("caption", cp["months_long"][v - 1], x + T["row_pad"], cy=cy, fill="accent")
                sc.line(x, cy + pitch / 2 - 2, x + cw, cy + pitch / 2 - 2, "rule", T["hair_w"])
            else:
                n = names[v]
                sc.text("bday_day", f"{n['d']:02d}", x + T["row_pad"], cy=cy, fill="inv_fg" if n["hl"] else None)
                sc.text("name_hl" if n["hl"] else "name", shown[v], x + T["row_pad"] + dw + T["day_gap"], cy=cy, size=size, fill="inv_fg" if n["hl"] else None)
    sc.text("kicker", D["offerings"]["date_long"], T["mx"], cy=T["meta_bot_cy"], fill="mute")
    return sc

def majalah_S03(spec, fonts):
    sc, T, D = new(spec, "majalah", fonts)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    majalah_page(sc, T, O["title"], O["date_long"], spec)
    xr = sc.W - T["mx"]
    sc.text("caption", cp["fund_head"], T["mx"], cy=T["head_cy"])
    sc.text("caption", cp["amount_head"], xr, cy=T["head_cy"], anchor="end")
    sc.rect(T["mx"], T["head_rule_y"], xr - T["mx"], T["rule_thin"], "ink")
    n = len(funds)
    pitch = (T["rows_bot"] - T["rows_top"]) / n
    sc.fits["pitch"] = r2(pitch)
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - amount_w - T["lab_gap"] - T["mx"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["rows_top"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 10)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, T["mx"], cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr, cy, f["rp"])
        if i + 1 < n:
            sc.line(T["mx"], cy + pitch / 2, xr, cy + pitch / 2, "rule", T["hair_w"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    sc.rect(T["mx"], T["total_rule_y"], xr - T["mx"], T["rule_thick"], "ink")
    sc.rect(T["mx"], T["total_rule_y"] + T["rule_thick"] + T["rule_gap"], xr - T["mx"], T["rule_thin"], "ink")
    cy = T["total_cy"]
    sc.text("total_label", cp["total_gifts"], T["mx"], cy=cy)
    w = sc.text("total", rupiah(total), xr, cy=cy, anchor="end")
    sc.text("rp", cp["rp"], xr - w - 18, cy=cy, anchor="end", size=T["total_rp"])
    sc.fits["total_px"] = r2(w)
    return sc


# ------------------------------------------------------------------ 7 angka

def angka_head(sc, T, title, sub, tag):
    sc.rect(0, 0, sc.W, sc.H, "ground")
    sc.text("kicker", sc.fam["copy_kicker"], T["mx"], cy=T["kicker_cy"])
    w = sc.text("title", title, T["mx"], base=T["title_base"])
    sc.text("sub", sub, T["mx"] + w + T["sub_gap"], base=T["title_base"])
    if tag:
        xr = sc.W - T["mx"]
        tw = sc.w("tag", tag)
        sc.rect(xr - tw - 2 * T["tag_pad"], T["tag_cy"] - T["tag_h"] / 2, tw + 2 * T["tag_pad"], T["tag_h"], "signal")
        sc.text("tag", tag, xr - T["tag_pad"], cy=T["tag_cy"], anchor="end")


def week_of(spec, n):
    """A name's week on the page, counted from the page's first Sunday: 0, 1, 2 ..."""
    y, m, d = spec["data"]["birthdays"]["week_start"]
    return (datetime.date(y, n["m"], n["d"]) - datetime.date(y, m, d)).days // 7


def angka_S02(spec, fonts):
    sc, T, D = new(spec, "angka", fonts)
    sc.fam["copy_kicker"] = kicker_text(spec, sc)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    angka_head(sc, T, cp["birthdays"], B["month_label"], cp["week_this"] + " " + B["week_label"])
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    bar = T["week_bar"]
    dw = max(T["day_w"], max(sc.w("bday_day", f"{n['d']:02d}") + T["mon_gap"] + sc.w("date", cp["months_short"][n["m"] - 1].upper()) for n in names) + 2 * T["day_pad"]) + bar
    size, shown = fit_names(sc, names, "name", "name_hl", cw - dw - T["name_pad"], T["name_floor"])
    sc.fits["day_col"] = r2(dw)
    pitch = (T["body_bot"] - T["body_top"]) / per
    sc.fits["pitch"] = r2(pitch)
    hy = T["head_y"] + T["head_h"] / 2
    for c in range(3):
        x = T["mx"] + c * (cw + g)
        sc.text("caption", cp["day_head"], x + bar + T["day_pad"], cy=hy)
        sc.text("caption", cp["name_head"], x + dw + T["name_pad"], cy=hy)
        sc.rect(x, T["head_y"] + T["head_h"] - T["rule_head"], cw, T["rule_head"], "ink")
        sc.rect(x, T["body_top"], cw, T["body_bot"] - T["body_top"], "cell")
    weeks = []
    for c in range(3):
        col = names[c * per:(c + 1) * per]
        i = 0
        while i < len(col):
            k = i
            while k + 1 < len(col) and week_of(spec, col[k + 1]) == week_of(spec, col[i]):
                k += 1
            x = T["mx"] + c * (cw + g)
            wk = week_of(spec, col[i])
            sc.rect(x, T["body_top"] + i * pitch, bar, (k - i + 1) * pitch, f"fund{wk % 6 + 1}")
            weeks.append(wk)
            i = k + 1
    sc.fits["weeks"] = sorted(set(weeks))
    for c, i0, n in hl_runs(names, per):
        x = T["mx"] + c * (cw + g)
        sc.rect(x + bar, T["body_top"] + i0 * pitch, cw - bar, n * pitch, "signal", stroke="ink", sw=T["hl_sw"])
    for i, n in enumerate(names):
        c, r = divmod(i, per)
        x, cy = T["mx"] + c * (cw + g), T["body_top"] + (r + 0.5) * pitch
        first = r == 0 or (names[i - 1]["d"], names[i - 1]["m"]) != (n["d"], n["m"])
        if first:
            w = sc.text("bday_day", f"{n['d']:02d}", x + bar + T["day_pad"], cy=cy, fill="on_signal" if n["hl"] else None)
            sc.text("date", cp["months_short"][n["m"] - 1].upper(), x + bar + T["day_pad"] + w + T["mon_gap"], cy=cy, fill="on_signal" if n["hl"] else None)
        sc.text("name_hl" if n["hl"] else "name", shown[i], x + dw + T["name_pad"], cy=cy, size=size, fill="on_signal" if n["hl"] else None)
        if r + 1 < per and not n["hl"] and not names[i + 1]["hl"]:
            nxt_same = (names[i + 1]["d"], names[i + 1]["m"]) == (n["d"], n["m"])
            sc.line(x + (dw if nxt_same else bar), cy + pitch / 2, x + cw, cy + pitch / 2, "ink", T["rule_w"], op=T["rule_op"])
    return sc


def angka_S03(spec, fonts):
    sc, T, D = new(spec, "angka", fonts)
    sc.fam["copy_kicker"] = kicker_text(spec, sc)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    angka_head(sc, T, O["title"], O["date_long"], None)
    xr = sc.W - T["mx"]
    hy = T["head_y"] + T["head_h"] / 2
    top = T["head_y"] + T["head_h"]
    sc.text("caption", cp["no_head"], T["mx"] + T["no_w"] / 2, cy=hy, anchor="middle")
    sc.text("caption", cp["fund_head"], T["lab_x"], cy=hy)
    sc.text("caption", cp["share_head"], T["bar_x"], cy=hy)
    sc.text("caption", cp["amount_head"], xr - T["row_pad"], cy=hy, anchor="end")
    sc.rect(T["mx"], top - T["rule_head"], xr - T["mx"], T["rule_head"], "ink")
    n = len(funds)
    pitch = (T["rows_bot"] - top) / n
    sc.fits["pitch"] = r2(pitch)
    sc.rect(T["mx"], top, xr - T["mx"], T["rows_bot"] - top, "cell")
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = T["bar_x"] - T["lab_x"] - T["lab_gap"]
    lines_out, shares = [], []
    for i, f in enumerate(funds):
        cy = top + (i + 0.5) * pitch
        col = f"fund{i + 1}"
        sc.rect(T["mx"] + T["chip_pad"], cy - T["chip"] / 2, T["chip"], T["chip"], col)
        sc.text("bday_day", str(i + 1), T["mx"] + T["chip_pad"] + T["chip"] / 2, cy=cy, anchor="middle", fill="on_fund", size=T["chip_num"])
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 10)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, T["lab_x"], cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        share = f["rp"] / total
        shares.append(pct(f["rp"], total))
        sc.rect(T["bar_x"], cy - T["bar_h"] / 2, T["bar_w"], T["bar_h"], "ink", op=T["track_op"])
        sc.rect(T["bar_x"], cy - T["bar_h"] / 2, T["bar_w"] * share, T["bar_h"], col)
        sc.text("note", pct(f["rp"], total), T["bar_x"] + T["bar_w"] + T["pct_gap"], cy=cy, size=T["pct_size"], font="p7", fill="ink")
        money(sc, xr - T["row_pad"], cy, f["rp"])
        if i + 1 < n:
            sc.line(T["mx"], cy + pitch / 2, xr, cy + pitch / 2, "ink", T["rule_w"], op=T["rule_op"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total, shares=shares)
    sc.rect(0, T["total_top"], sc.W, sc.H - T["total_top"], "signal")
    cy = (T["total_top"] + sc.H) / 2
    sc.text("total_label", cp["total_gifts"], T["mx"], cy=cy)
    lw = sc.w("total_label", cp["total_gifts"])
    room = xr - T["mx"] - lw - T["tot_gap"] - T["total_rp_w"]
    size = T["total_max"]
    while sc.w("total", rupiah(total), size) > room and size > T["total_min"]:
        size -= 1
    w = sc.text("total", rupiah(total), xr, cy=cy, anchor="end", size=size)
    sc.text("total", cp["rp"], xr - w - 20, cy=cy, anchor="end", size=T["total_rp"])
    sc.fits.update(total_px=r2(w), total_size=size)
    return sc


# ------------------------------------------------------------------ 8 hangat

def hangat_ground(sc, T):
    sc.rect(0, 0, sc.W, sc.H, "ground")
    for a in T["arches"]:
        x, y, w, h = a["x"], a["y"], a["w"], a["h"]
        r = w / 2
        sc.add(dict(k="d", d=["M", x, y + h, "L", x, y + r, "A", r, r, 0, 0, 1, x + w, y + r, "L", x + w, y + h], st="ink", sw=T["arch_sw"], op=T["arch_op"]))


def hangat_title(sc, T, title, sub, spec):
    sc.text("kicker", kicker_text(spec, sc), T["mx"], cy=T["kicker_cy"])
    w = sc.text("title", title, T["mx"], base=T["title_base"])
    sc.text("sub", sub, T["mx"] + w + T["sub_gap"], base=T["title_base"])


def hangat_S02(spec, fonts):
    sc, T, D = new(spec, "hangat", fonts)
    B, cp = D["birthdays"], D["copy"]
    names = names_of(spec)
    hangat_ground(sc, T)
    hangat_title(sc, T, cp["birthdays"], B["month_label"], spec)
    label = cp["week_this"] + "  " + B["week_label"]
    xr = sc.W - T["mx"]
    tw = sc.w("tag", label)
    sc.rect(xr - tw - 2 * T["pill_pad"], T["pill_cy"] - T["pill_h"] / 2, tw + 2 * T["pill_pad"], T["pill_h"], "accent", rx=T["pill_h"] / 2)
    sc.text("tag", label, xr - T["pill_pad"], cy=T["pill_cy"], anchor="end")
    per, g = T["per"], T["gutter"]
    cw = (sc.W - 2 * T["mx"] - 2 * g) / 3
    dw = max(T["day_w"], max(sc.w("bday_day", f"{n['d']:02d}") for n in names))
    full = day_width(sc, cp, names, 0)
    nx = dw + T["day_gap"]
    cell = cw - nx - 2 * T["card_pad"]
    size, shown = fit_names(sc, names, "name", "name_hl", cell, T["name_floor"])
    sc.fits.update(day_col=r2(dw), shortened_if_each_row_names_its_month=alt_shortened(sc, names, "name", "name_hl", cell, size, full - dw))
    cols = month_cols(names, per)
    rows = max(len(c) for c in cols)
    pitch = (T["card_h"] - 2 * T["card_pad_v"]) / rows
    sc.fits["pitch"] = r2(pitch)
    for c in range(3):
        sc.rect(T["mx"] + c * (cw + g), T["card_y"], cw, T["card_h"], "card", rx=T["card_rx"])
    y0 = T["card_y"] + T["card_pad_v"]
    for c, j0, n in item_runs(names, cols):
        sc.rect(T["mx"] + c * (cw + g) + T["band_in"], y0 + j0 * pitch + 3, cw - 2 * T["band_in"], n * pitch - 6, "accent", rx=T["band_rx"])
    for c, items in enumerate(cols):
        x = T["mx"] + c * (cw + g)
        for j, (kind, v) in enumerate(items):
            cy = y0 + (j + 0.5) * pitch
            if kind == "h":
                sc.text("sub", cp["months_long"][v - 1], x + T["card_pad"], cy=cy, size=T["head_size"])
            else:
                n = names[v]
                sc.text("bday_day", f"{n['d']:02d}", x + T["card_pad"], cy=cy, fill="on_accent" if n["hl"] else None)
                sc.text("name_hl" if n["hl"] else "name", shown[v], x + T["card_pad"] + nx, cy=cy, size=size, fill="on_accent" if n["hl"] else None)
    return sc

def hangat_S03(spec, fonts):
    sc, T, D = new(spec, "hangat", fonts)
    O, cp = D["offerings"], D["copy"]
    funds = funds_of(spec)
    total = sum(f["rp"] for f in funds)
    hangat_ground(sc, T)
    hangat_title(sc, T, O["title"], O["date_long"], spec)
    xl, xr = T["mx"], sc.W - T["mx"]
    sc.rect(xl, T["card_y"], xr - xl, T["card_h3"], "card", rx=T["card_rx"])
    n = len(funds)
    pad = T["card_pad"]
    pitch = (T["card_h3"] - 2 * T["card_pad_v"]) / n
    sc.fits["pitch"] = r2(pitch)
    amount_w = max(sc.w("amount", rupiah(f["rp"])) for f in funds) + 12 + sc.w("rp", cp["rp"])
    avail = xr - xl - 2 * pad - amount_w - T["lab_gap"]
    lines_out = []
    for i, f in enumerate(funds):
        cy = T["card_y"] + T["card_pad_v"] + (i + 0.5) * pitch
        lines, size = fit_label(sc, "fund", f["label"], avail, T["label_floor"], pitch - 10)
        lines_out.append(len(lines))
        for k, ln in enumerate(lines):
            sc.text("fund", ln, xl + pad, cy=cy + (k - (len(lines) - 1) / 2) * LINE * size, size=size)
        money(sc, xr - pad, cy, f["rp"])
        if i + 1 < n:
            sc.line(xl + pad, cy + pitch / 2, xr - pad, cy + pitch / 2, "line", T["rule_w"])
    sc.fits.update(rows=n, label_lines=lines_out, amount_col=r2(amount_w), total=total)
    sc.rect(xl + T["plate_in"], T["plate_y"], xr - xl - T["plate_in"], T["plate_h"], "plate", rx=T["card_rx"])
    cy = T["plate_y"] + T["plate_h"] / 2
    sc.text("total_label", cp["total_gifts"], xl + T["plate_in"] + T["plate_pad"], cy=cy)
    w = sc.text("total", rupiah(total), xr - T["plate_pad"], cy=cy, anchor="end")
    sc.text("rp", cp["rp"], xr - T["plate_pad"] - w - 18, cy=cy, anchor="end", size=T["total_rp"], fill="on_plate")
    sc.fits["total_px"] = r2(w)
    return sc


LAYOUTS = {
    "diagonal": (diagonal_S02, diagonal_S03), "liturgi": (liturgi_S02, liturgi_S03), "kaca": (kaca_S02, kaca_S03), "blok": (blok_S02, blok_S03),
    "cahaya": (cahaya_S02, cahaya_S03), "majalah": (majalah_S02, majalah_S03), "angka": (angka_S02, angka_S03), "hangat": (hangat_S02, hangat_S03),
}
