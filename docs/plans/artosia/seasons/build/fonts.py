"""Text measure for the wireframes, from the bundled faces (crates/artosia-slides/assets/warta/fonts).

Advance widths from the hmtx table plus tracking; no kerning and no shaping, so a measured width is within a few percent of what the
engine's text layout gives. Stated in spec.json "meta"; the real builder measures with the engine's own `Sheet::text`.
"""
import os

from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
FONT_DIR = os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "..", "crates", "artosia-slides", "assets", "warta", "fonts"))
_cache = {}


def face(file):
    if file not in _cache:
        _cache[file] = TTFont(os.path.join(FONT_DIR, file))
    return _cache[file]


def advance(file, text, size, ls_em=0.0):
    f = face(file)
    cmap, hmtx, upm = f.getBestCmap(), f["hmtx"], f["head"].unitsPerEm
    w = 0.0
    for ch in text:
        g = cmap.get(ord(ch))
        w += (hmtx[g][0] if g else upm * 0.5) / upm * size + ls_em * size
    return w


def cap_height(file, size):
    f = face(file)
    return f["OS/2"].sCapHeight / f["head"].unitsPerEm * size
