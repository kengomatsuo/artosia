"""Cut the static fonts the four candidates draw with, from SIL OFL sources (github.com/google/fonts, ofl/ folder).

python -I make_fonts.py FONT_DIR

Why static: resvg, the PNG renderer, ignores variable axes and font-feature-settings, so every weight, optical size and
digit style used on a wireframe is one file. Digits are baked tabular by writing the 'tnum' substitution into the cmap,
because Artosia asks Rune for the same feature (`TextStyle::features("'tnum' 1")`, crates/artosia-slides/src/notice.rs).
FONT_DIR/src holds the sources (fetched when missing); FONT_DIR/static holds the cuts the build reads.
"""
import os, sys, urllib.request
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

RAW = "https://raw.githubusercontent.com/google/fonts/main/ofl/"
SOURCES = {  # file -> path under ofl/
    "SourceSerif4[opsz,wght].ttf": "sourceserif4/SourceSerif4%5Bopsz%2Cwght%5D.ttf",
    "SourceSerif4-Italic[opsz,wght].ttf": "sourceserif4/SourceSerif4-Italic%5Bopsz%2Cwght%5D.ttf",
    "HankenGrotesk[wght].ttf": "hankengrotesk/HankenGrotesk%5Bwght%5D.ttf",
    "Figtree[wght].ttf": "figtree/Figtree%5Bwght%5D.ttf",
    "IBMPlexMono-Regular.ttf": "ibmplexmono/IBMPlexMono-Regular.ttf",
    "IBMPlexMono-Medium.ttf": "ibmplexmono/IBMPlexMono-Medium.ttf",
    "IBMPlexMono-SemiBold.ttf": "ibmplexmono/IBMPlexMono-SemiBold.ttf",
    "IBMPlexMono-Bold.ttf": "ibmplexmono/IBMPlexMono-Bold.ttf",
    "IBMPlexSansCondensed-Medium.ttf": "ibmplexsanscondensed/IBMPlexSansCondensed-Medium.ttf",
    "IBMPlexSansCondensed-SemiBold.ttf": "ibmplexsanscondensed/IBMPlexSansCondensed-SemiBold.ttf",
    "IBMPlexSansCondensed-Bold.ttf": "ibmplexsanscondensed/IBMPlexSansCondensed-Bold.ttf",
}
SS, SSI = "SourceSerif4[opsz,wght].ttf", "SourceSerif4-Italic[opsz,wght].ttf"
HG, FT = "HankenGrotesk[wght].ttf", "Figtree[wght].ttf"

# (family, source, weight, italic, axes, baked feature tags)
ROSTER = [
    ("Source Serif 4 Display", SS, 600, False, {"wght": 600, "opsz": 60}, ()),
    ("Source Serif 4 Display", SS, 400, False, {"wght": 400, "opsz": 60}, ()),
    ("Source Serif 4 Display", SSI, 400, True, {"wght": 400, "opsz": 48}, ()),
    ("Source Serif 4 Text", SS, 400, False, {"wght": 400, "opsz": 24}, ()),
    ("Source Serif 4 Text", SS, 600, False, {"wght": 600, "opsz": 24}, ()),
    ("Source Serif 4 Text", SSI, 400, True, {"wght": 400, "opsz": 24}, ()),
    ("Source Serif 4 Oldstyle", SSI, 400, True, {"wght": 400, "opsz": 48}, ("onum",)),
    ("Hanken Grotesk", HG, 300, False, {"wght": 300}, ("tnum",)),
    ("Hanken Grotesk", HG, 500, False, {"wght": 500}, ("tnum",)),
    ("Hanken Grotesk", HG, 700, False, {"wght": 700}, ("tnum",)),
    ("Figtree", FT, 500, False, {"wght": 500}, ("tnum",)),
    ("Figtree", FT, 700, False, {"wght": 700}, ("tnum",)),
    ("IBM Plex Mono", "IBMPlexMono-Regular.ttf", 400, False, None, ()),
    ("IBM Plex Mono", "IBMPlexMono-Medium.ttf", 500, False, None, ()),
    ("IBM Plex Mono", "IBMPlexMono-SemiBold.ttf", 600, False, None, ()),
    ("IBM Plex Mono", "IBMPlexMono-Bold.ttf", 700, False, None, ()),
    ("IBM Plex Sans Condensed", "IBMPlexSansCondensed-Medium.ttf", 500, False, None, ()),
    ("IBM Plex Sans Condensed", "IBMPlexSansCondensed-SemiBold.ttf", 600, False, None, ()),
    ("IBM Plex Sans Condensed", "IBMPlexSansCondensed-Bold.ttf", 700, False, None, ()),
]


def bake(font, tag):
    """Map the digits to their `tag` glyphs (tnum, onum); True when the font has the substitution."""
    gsub = font["GSUB"].table if "GSUB" in font else None
    if gsub is None:
        return False
    wanted = set()
    for rec in gsub.FeatureList.FeatureRecord:
        if rec.FeatureTag == tag:
            wanted.update(rec.Feature.LookupListIndex)
    sub = {}
    for at in wanted:
        for st in gsub.LookupList.Lookup[at].SubTable:
            st = getattr(st, "ExtSubTable", st)
            sub.update(getattr(st, "mapping", {}))
    cmap, hit = font.getBestCmap(), False
    for cp in range(0x30, 0x3A):
        glyph = cmap.get(cp)
        if glyph in sub:
            hit = True
            for table in font["cmap"].tables:
                if cp in table.cmap:
                    table.cmap[cp] = sub[glyph]
    return hit


def digit_advances(font):
    cmap, hmtx = font.getBestCmap(), font["hmtx"]
    return sorted({hmtx[cmap[cp]][0] for cp in range(0x30, 0x3A)})


def main(root):
    src, out = os.path.join(root, "src"), os.path.join(root, "static")
    os.makedirs(src, exist_ok=True)
    os.makedirs(out, exist_ok=True)
    for name, path in SOURCES.items():
        if not os.path.exists(os.path.join(src, name)):
            urllib.request.urlretrieve(RAW + path, os.path.join(src, name))
            print("fetched", name)
    for family, source, weight, italic, axes, tags in ROSTER:
        style = ("Italic" if italic else "Regular") if weight == 400 else (str(weight) + (" Italic" if italic else ""))
        file = os.path.join(out, f"{family.replace(' ', '')}-{weight}{'i' if italic else ''}.ttf")
        font = TTFont(os.path.join(src, source))
        if axes:
            font = instancer.instantiateVariableFont(font, axes, inplace=False)
        baked = [tag for tag in tags if bake(font, tag)]
        table = font["name"]
        for rec in list(table.names):
            if rec.nameID in (1, 2, 3, 4, 6, 16, 17, 21, 22, 25):
                table.removeNames(nameID=rec.nameID)
        for platform in ((3, 1, 0x409), (1, 0, 0)):
            table.setName(family, 1, *platform)
            table.setName(style, 2, *platform)
            table.setName(f"{family} {style}", 4, *platform)
            table.setName(f"{family}-{style}".replace(" ", ""), 6, *platform)
        font["OS/2"].usWeightClass = weight
        if italic:
            font["OS/2"].fsSelection = (font["OS/2"].fsSelection | 1) & ~64
            font["head"].macStyle |= 2
        font.save(file)
        print(f"{os.path.basename(file):44s} baked={baked!s:10s} digit advances={digit_advances(TTFont(file))}")


if __name__ == "__main__":
    main(sys.argv[1])
