"""Lays out every PNG of a folder whose names share a part before "__" as one labelled sheet.

    python -I tools/contact-sheet.py <png folder> <sheet folder> [columns]
"""
import os
import sys

from PIL import Image, ImageDraw

source, target = sys.argv[1], sys.argv[2]
columns = int(sys.argv[3]) if len(sys.argv) > 3 else 4
os.makedirs(target, exist_ok=True)
groups = {}
for name in sorted(os.listdir(source)):
    if name.endswith(".png"):
        groups.setdefault(name.split("__")[0], []).append(name)
for group, names in groups.items():
    tiles = [Image.open(os.path.join(source, name)).convert("RGB") for name in names]
    w, h = tiles[0].size
    label = 28
    rows = (len(tiles) + columns - 1) // columns
    sheet = Image.new("RGB", (columns * (w + 8) + 8, rows * (h + label + 8) + 8), (40, 40, 40))
    draw = ImageDraw.Draw(sheet)
    for at, (name, tile) in enumerate(zip(names, tiles)):
        x, y = 8 + (at % columns) * (w + 8), 8 + (at // columns) * (h + label + 8)
        draw.text((x, y + 6), name[len(group) + 2:-4], fill=(230, 230, 230))
        sheet.paste(tile, (x, y + label))
    sheet.save(os.path.join(target, f"{group}.png"))
    print(group, len(tiles))
