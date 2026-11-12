"""Writes crates/artosia-core/assets/languages.json from Unicode CLDR's JSON.

    python -I tools/make-languages.py [<folder with the CLDR files>]

The folder is the research folder by default (`~/Shared/research/artosia-multilingual/cldr`),
which holds files fetched from unicode-org/cldr-json tag 49.0.0 (see its notes.md):

  cldr-territoryInfo.json       cldr-core/supplemental/territoryInfo.json
  cldr-aliases.json             cldr-core/supplemental/aliases.json
  cldr-languages-<locale>.json  cldr-localenames-full/main/<locale>/languages.json
  cldr-scripts-<locale>.json, cldr-localeDisplayNames-<locale>.json
                                the same folder's scripts.json and localeDisplayNames.json, for the
                                locales whose languages.json has no short name for Chinese in one
                                script (id, ko, vi)

The asset has the shape the program reads (`crates/artosia-core/src/languages.rs`):

  {"cldr": "49.0.0",
   "names": {"en": {"id": "Indonesian", "zh-Hans": "Simplified Chinese", ...}, "id": {...}, ...},
   "iso3": {"ind": "id", "zsm": "ms", ...}}

`names` is keyed by the window's language code (pt-BR reads CLDR's pt) and holds only the
languages a church might sing in: those CLDR's territory data lists as spoken somewhere, and
Chinese in its two scripts. Where a locale has no short name for Chinese in one script, the name
is put together as CLDR does it: the locale's name for Chinese and for the script in the locale's
own pattern ("Tionghoa (Sederhana)"). A language CLDR has no name for in a window language is
left out of that language's table; the program falls back to English. Codes are BCP 47: the two-letter code
where one exists, else the three-letter one. `iso3` maps the three-letter codes a country's
language list (`countries.json`) uses to those codes.
"""
import json
import os
import sys

TAG = "49.0.0"
LOCALES = {"en": "en", "id": "id", "zh-Hans": "zh-Hans", "zh-Hant": "zh-Hant", "ko": "ko", "ja": "ja", "es": "es", "pt-BR": "pt", "fr": "fr", "vi": "vi"}
# Members eBible and Beblia write instead of the macrolanguage.
MEMBERS = {"zsm": "ms", "zlm": "ms", "msa": "ms", "cmn": "zh-Hans", "zho": "zh-Hans", "arb": "ar", "ara": "ar"}


def read(folder, name):
    with open(os.path.join(folder, name), encoding="utf-8") as f:
        return json.load(f)


def main():
    folder = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/Shared/research/artosia-multilingual/cldr")
    info = read(folder, "cldr-territoryInfo.json")["supplemental"]["territoryInfo"]
    english = read(folder, "cldr-languages-en.json")["main"]["en"]["localeDisplayNames"]["languages"]
    spoken = set()
    for territory in info.values():
        for tag, share in territory.get("languagePopulation", {}).items():
            if float(share.get("_populationPercent", "0")) > 0:
                spoken.add(tag.split("_")[0])
    # Bare Chinese is ambiguous in a song: the two scripts stand for it.
    codes = sorted((spoken - {"und", "zh"}) | {"zh-Hans", "zh-Hant"})
    codes = [code for code in codes if code in english]
    names = {}
    for window, locale in LOCALES.items():
        raw = read(folder, "cldr-languages-%s.json" % locale)["main"][locale]["localeDisplayNames"]["languages"]
        names[window] = {code: raw[code] for code in codes if code in raw}
        for code, script in (("zh-Hans", "Hans"), ("zh-Hant", "Hant")):
            if code not in names[window] and os.path.exists(os.path.join(folder, "cldr-scripts-%s.json" % locale)):
                scripts = read(folder, "cldr-scripts-%s.json" % locale)["main"][locale]["localeDisplayNames"]["scripts"]
                pattern = read(folder, "cldr-localeDisplayNames-%s.json" % locale)["main"][locale]["localeDisplayNames"]["localeDisplayPattern"]["localePattern"]
                names[window][code] = pattern.replace("{0}", raw["zh"]).replace("{1}", scripts[script])
    iso3 = {}
    for long, entry in read(folder, "cldr-aliases.json")["supplemental"]["metadata"]["alias"]["languageAlias"].items():
        replacement = entry.get("_replacement", "")
        if entry.get("_reason") == "overlong" and len(long) == 3 and replacement in codes:
            iso3[long] = replacement
    iso3.update(MEMBERS)
    out = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "crates", "artosia-core", "assets", "languages.json")
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"cldr": TAG, "names": names, "iso3": iso3}, f, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        f.write("\n")
    print("%d languages, %d iso3 codes, %d bytes" % (len(codes), len(iso3), os.path.getsize(out)))


main()
