"""Contact sheets: every candidate's schedule and birthday wireframes on one PNG, and each candidate's two kinds in three colourways on another.

The PNGs are rendered from the SVGs by resvg (bun x @resvg/resvg-js-cli, system fonts off, the cut fonts in FONT_DIR/static).
"""
import os
from PIL import Image, ImageDraw, ImageFont

import engine

TW, TH, GAP, LABEL = 624, 351, 16, 74


def font(size, weight="700"):
    return ImageFont.truetype(os.path.join(engine.static_dir(), f"HankenGrotesk-{weight}.ttf"), size)


def grid(rows, cols, path, title_of, cell_of, caption_of, heads=()):
    w = GAP + cols * (TW + GAP)
    top = 56
    h = top + GAP + len(rows) * (LABEL + TH + GAP)
    im = Image.new("RGB", (w, h), (236, 235, 231))
    d = ImageDraw.Draw(im)
    for ci, hd in enumerate(heads):
        d.text((GAP + ci * (TW + GAP), 14), hd, fill=(20, 20, 20), font=font(26))
    for ri, r in enumerate(rows):
        y = top + GAP + ri * (LABEL + TH + GAP)
        d.text((GAP, y + 6), title_of(r), fill=(20, 20, 20), font=font(30))
        d.text((GAP, y + 44), caption_of(r), fill=(90, 90, 86), font=font(20, "500"))
        for ci in range(cols):
            a = Image.open(cell_of(r, ci)).convert("RGB").resize((TW, TH), Image.LANCZOS)
            x = GAP + ci * (TW + GAP)
            im.paste(a, (x, y + LABEL))
            d.rectangle((x - 1, y + LABEL - 1, x + TW, y + LABEL + TH), outline=(150, 150, 146))
    im.save(path)
    return im.size


def run(spec, png_dir, root):
    cands = list(spec["resolved"])
    kinds = [("schedule", "Schedule"), ("birthdays", "Birthdays, 30 names"), ("birthdays-two-months-1of2", "Birthdays, two months, 1/2")]
    cap = lambda c: spec["candidates"][c]["idea"]
    size = grid(cands, 3, os.path.join(root, "contact-sheet.png"),
                lambda c: f"{c.upper()}  {spec['candidates'][c]['name']}",
                lambda c, i: os.path.join(png_dir, f"{c}-{kinds[i][0]}.png"), cap, [k[1] for k in kinds])
    print("contact-sheet.png", size)
    size = grid(cands, 3, os.path.join(root, "colourways-sheet.png"),
                lambda c: f"{c.upper()}  {spec['candidates'][c]['name']}",
                lambda c, i: os.path.join(png_dir, f"{c}-{spec['candidates'][c]['colourways'][i]['id']}-schedule.png"), cap, ["Light", "Dark", "Tinted"])
    print("colourways-sheet.png", size)
