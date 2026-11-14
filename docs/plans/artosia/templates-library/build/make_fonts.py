"""Cut the static fonts the eight library families draw with, from SIL OFL sources (github.com/google/fonts, ofl/ folder).

python -I make_fonts.py SRC_DIR OUT_DIR REPO_FONTS_DIR

SRC_DIR holds the downloaded sources (names in SOURCES); REPO_FONTS_DIR is crates/artosia-slides/assets/warta/fonts, whose
bundled cuts are copied unchanged; OUT_DIR gets one static file per (family, weight, slant): <Family>-<weight>[i].ttf.
Static because the wireframes are measured with fontTools and drawn from the same files, and Artosia bundles static cuts.
Digits are made tabular by writing the 'tnum' substitution into the cmap, as Artosia asks Rune for 'tnum'. Prints each
cut's size and its digit advances (one value: tabular; several: proportional).
"""
import os, shutil, sys
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

SOURCES = {
    "montserrat/Montserrat[wght].ttf": "Montserrat[wght].ttf",
    "cinzel/Cinzel[wght].ttf": "Cinzel[wght].ttf",
    "sourceserif4/SourceSerif4[opsz,wght].ttf": "SourceSerif4[opsz,wght].ttf",
    "jost/Jost[wght].ttf": "Jost[wght].ttf",
    "allura/Allura-Regular.ttf": "Allura-Regular.ttf",
    "unbounded/Unbounded[wght].ttf": "Unbounded[wght].ttf",
    "sora/Sora[wght].ttf": "Sora[wght].ttf",
    "librecaslondisplay/LibreCaslonDisplay-Regular.ttf": "LibreCaslonDisplay-Regular.ttf",
    "librecaslontext/LibreCaslonText[wght].ttf": "LibreCaslonText[wght].ttf",
    "librecaslontext/LibreCaslonText-Italic[wght].ttf": "LibreCaslonText-Italic[wght].ttf",
    "bigshouldersdisplay/BigShouldersDisplay[wght].ttf": "BigShouldersDisplay[wght].ttf",
    "fraunces/Fraunces[SOFT,WONK,opsz,wght].ttf": "Fraunces[SOFT,WONK,opsz,wght].ttf",
    "fraunces/Fraunces-Italic[SOFT,WONK,opsz,wght].ttf": "Fraunces-Italic[SOFT,WONK,opsz,wght].ttf",
}
# (output stem, source file, axes to pin or None for a static source)
CUTS = [
    ("Montserrat-700", "Montserrat[wght].ttf", {"wght": 700}),
    ("Montserrat-800", "Montserrat[wght].ttf", {"wght": 800}),
    ("Cinzel-600", "Cinzel[wght].ttf", {"wght": 600}),
    ("Cinzel-700", "Cinzel[wght].ttf", {"wght": 700}),
    ("SourceSerif4Text-600", "SourceSerif4[opsz,wght].ttf", {"wght": 600, "opsz": 24}),
    ("Jost-300", "Jost[wght].ttf", {"wght": 300}),
    ("Jost-500", "Jost[wght].ttf", {"wght": 500}),
    ("Jost-600", "Jost[wght].ttf", {"wght": 600}),
    ("Allura-400", "Allura-Regular.ttf", None),
    ("Unbounded-700", "Unbounded[wght].ttf", {"wght": 700}),
    ("Unbounded-800", "Unbounded[wght].ttf", {"wght": 800}),
    ("Sora-300", "Sora[wght].ttf", {"wght": 300}),
    ("Sora-600", "Sora[wght].ttf", {"wght": 600}),
    ("LibreCaslonDisplay-400", "LibreCaslonDisplay-Regular.ttf", None),
    ("LibreCaslonText-400", "LibreCaslonText[wght].ttf", {"wght": 400}),
    ("LibreCaslonText-700", "LibreCaslonText[wght].ttf", {"wght": 700}),
    ("LibreCaslonText-400i", "LibreCaslonText-Italic[wght].ttf", {"wght": 400}),
    ("BigShouldersDisplay-800", "BigShouldersDisplay[wght].ttf", {"wght": 800}),
    ("BigShouldersDisplay-900", "BigShouldersDisplay[wght].ttf", {"wght": 900}),
    ("Fraunces-400", "Fraunces[SOFT,WONK,opsz,wght].ttf", {"wght": 400, "opsz": 48, "SOFT": 50, "WONK": 0}),
    ("Fraunces-600", "Fraunces[SOFT,WONK,opsz,wght].ttf", {"wght": 600, "opsz": 48, "SOFT": 50, "WONK": 0}),
    ("Fraunces-400i", "Fraunces-Italic[SOFT,WONK,opsz,wght].ttf", {"wght": 400, "opsz": 48, "SOFT": 50, "WONK": 0}),
]
BUNDLED = ["BarlowSemiCondensed-500", "BarlowSemiCondensed-600", "BarlowSemiCondensed-700",
           "IBMPlexSansCondensed-500", "IBMPlexSansCondensed-600", "IBMPlexSansCondensed-700",
           "InstrumentSans-500", "InstrumentSans-600", "InstrumentSans-700", "SourceSerif4Text-400"]


def bake_tnum(font):
    """Map the digits to their tnum glyphs; True when the font has the substitution."""
    gsub = font["GSUB"].table if "GSUB" in font else None
    if gsub is None:
        return False
    wanted = {i for rec in gsub.FeatureList.FeatureRecord if rec.FeatureTag == "tnum" for i in rec.Feature.LookupListIndex}
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


def main(src, out, repo_fonts):
    os.makedirs(out, exist_ok=True)
    for stem, source, axes in CUTS:
        font = TTFont(os.path.join(src, source))
        if axes:
            font = instancer.instantiateVariableFont(font, axes, inplace=False)
        baked = bake_tnum(font)
        font.save(os.path.join(out, stem + ".ttf"))
        print(f"{stem + '.ttf':36s} {os.path.getsize(os.path.join(out, stem + '.ttf')):8d} B tnum_baked={baked!s:5s} digit advances={digit_advances(TTFont(os.path.join(out, stem + '.ttf')))}")
    for stem in BUNDLED:
        shutil.copy(os.path.join(repo_fonts, stem + ".ttf"), os.path.join(out, stem + ".ttf"))
        print(f"{stem + '.ttf':36s} bundled   digit advances={digit_advances(TTFont(os.path.join(out, stem + '.ttf')))}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
