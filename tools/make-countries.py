"""Writes crates/artosia-core/assets/countries.json from Unicode CLDR's JSON.

    python -I tools/make-countries.py [<folder with the CLDR files>]

The folder is the research folder by default (`~/Shared/research/artosia-onboarding/country`),
which holds files fetched from unicode-org/cldr-json tag 49.0.0 (see its notes.md):

  cldr-territoryInfo.json        cldr-core/supplemental/territoryInfo.json
  cldr-aliases.json              cldr-core/supplemental/aliases.json
  cldr-territories-<locale>.json cldr-localenames-full/main/<locale>/territories.json

The asset has the shape the program reads (`crates/artosia-core/src/countries.rs`):

  {"cldr": "49.0.0",
   "names": {"en": {"ID": "Indonesia", ...}, "id": {...}, ...},
   "languages": {"ID": ["ind", "jav", ...], ...}}

`names` is keyed by the window's language code (pt-BR reads CLDR's pt). `languages` lists each
country's languages as ISO 639-3 codes, most spoken first, as eBible and Beblia write them.
Only real countries are kept: two capital letters, and none of CLDR's other two-letter places
(the European Union, the United Nations, exceptional reservations such as AC, and ZZ).
"""
import json
import os
import sys

TAG = "49.0.0"
# CLDR's locale file for each window language, in the order the program lists them.
LOCALES = {"en": "en", "id": "id", "zh-Hans": "zh-Hans", "zh-Hant": "zh-Hant", "ko": "ko", "ja": "ja", "es": "es", "pt-BR": "pt", "fr": "fr", "vi": "vi"}
# Two letters CLDR gives to places that are not countries.
NOT_COUNTRIES = {"ZZ", "EU", "EZ", "UN", "QO", "XA", "XB", "AC", "CP", "DG", "EA", "IC", "TA"}
# eBible and Beblia write the members of these macrolanguages, not the macrolanguage itself.
MEMBERS = {"ms": ["msa", "zsm", "zlm"], "zh": ["zho", "cmn"], "ar": ["ara", "arb"]}


def read(folder, name):
    with open(os.path.join(folder, name), encoding="utf-8") as f:
        return json.load(f)


def three_letter(aliases):
    """Two-letter code to three-letter code, from the aliases CLDR keeps for the long forms."""
    out = {}
    for long, entry in aliases["supplemental"]["metadata"]["alias"]["languageAlias"].items():
        replacement = entry.get("_replacement", "")
        if entry.get("_reason") == "overlong" and len(long) == 3 and len(replacement) == 2:
            out[replacement] = long
    return out


def main():
    folder = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/Shared/research/artosia-onboarding/country")
    info = read(folder, "cldr-territoryInfo.json")["supplemental"]["territoryInfo"]
    to_three = three_letter(read(folder, "cldr-aliases.json"))
    names = {}
    for window, locale in LOCALES.items():
        raw = read(folder, "cldr-territories-%s.json" % locale)["main"][locale]["localeDisplayNames"]["territories"]
        names[window] = {code: name for code, name in raw.items() if len(code) == 2 and code.isalpha() and code.isupper() and code not in NOT_COUNTRIES}
    countries = sorted(names["en"])
    for window in names:
        assert sorted(names[window]) == countries, "the locales name different places: " + window
    languages = {}
    for code in countries:
        spoken = []
        for tag, share in info.get(code, {}).get("languagePopulation", {}).items():
            percent = float(share.get("_populationPercent", "0"))
            if percent <= 0:
                continue
            base = tag.split("_")[0]
            # A script or a variant (ms_Arab, zh_Hant) is the same language for a Bible list.
            spoken.append((percent, base))
        by_language = {}
        for percent, base in spoken:
            by_language[base] = by_language.get(base, 0) + percent
        ordered = []
        for base, _ in sorted(by_language.items(), key=lambda item: (-item[1], item[0])):
            for three in MEMBERS.get(base) or [to_three.get(base, base)]:
                if len(three) == 3 and three not in ordered:
                    ordered.append(three)
        if ordered:
            languages[code] = ordered
    out = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "crates", "artosia-core", "assets", "countries.json")
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"cldr": TAG, "names": names, "languages": languages}, f, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        f.write("\n")
    print("%d countries, %d with languages, %d bytes" % (len(countries), len(languages), os.path.getsize(out)))


main()
