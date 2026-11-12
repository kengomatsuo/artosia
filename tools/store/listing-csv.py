"""Fills Partner Center's listing template with Artosia's Store texts in every language.

    python -I tools/store/listing-csv.py <exported template .csv> <out .csv>

Partner Center exports a .csv whose first three columns (Field, ID, Type) must come back
unchanged; each further column is one language. This writes the template's rows with a
column for each language of `tools/store/listing/<code>.json`, so ten listings are imported
at once (Microsoft's "Import and export Store listings" page, read 2026-11-12). Pictures are
uploaded on each listing's own page: an import of a folder needs a folder picker.
"""
import csv
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Artosia's code for a language, and the Store's code for the same (its list of supported
# language codes, read 2026-11-12).
LANGUAGES = [
    ("en", "en-us"),
    ("id", "id"),
    ("zh-Hans", "zh-hans"),
    ("zh-Hant", "zh-hant"),
    ("ko", "ko"),
    ("ja", "ja"),
    ("es", "es"),
    ("pt-BR", "pt-br"),
    ("fr", "fr"),
    ("vi", "vi"),
]
CAPTIONS = ["caption_window", "caption_search", "caption_design", "caption_screens", "caption_notices", "caption_first_run"]
SHARED = {
    "CopyrightTrademarkInformation": "Copyright (C) 2026 Kenneth Johannes Fang. GPL-3.0-or-later",
    "AdditionalLicenseTerms": (
        "Artosia is free software under the GNU General Public License, version 3 or later. "
        "Its source code is at https://github.com/kengomatsuo/artosia. "
        "It bundles GStreamer (LGPL) and other libraries listed in the repository."
    ),
}


def fields(listing: dict) -> dict:
    out = {
        "Title": "Artosia",
        "Description": listing["description"],
        "ReleaseNotes": listing["whats_new"],
        "ShortDescription": listing["short_description"],
    }
    for n in range(1, 9):
        out[f"Feature{n}"] = listing[f"feature_{n}"]
    terms = [term.strip() for term in listing["keywords"].replace("，", ",").replace("、", ",").split(",") if term.strip()]
    if len(terms) > 7:
        sys.exit(f"more than seven search terms: {terms}")
    for n, term in enumerate(terms, 1):
        out[f"SearchTerm{n}"] = term
    for n, key in enumerate(CAPTIONS, 1):
        out[f"DesktopScreenshotCaption{n}"] = listing[key]
    return out


def main() -> None:
    template, out = Path(sys.argv[1]), Path(sys.argv[2])
    rows = [row for row in csv.reader(template.open(encoding="utf-8-sig", newline="")) if any(cell.strip() for cell in row)]
    head, body = rows[0], rows[1:]
    if [cell.strip() for cell in head[:3]] != ["Field", "ID", "Type (Type)"]:
        sys.exit(f"not a listing template: {head[:4]}")
    listings = {store: fields(json.loads((HERE / "listing" / f"{code}.json").read_text(encoding="utf-8"))) for code, store in LANGUAGES}
    known = {row[0] for row in body}
    for store, listing in listings.items():
        missing = sorted(set(listing) - known)
        if missing:
            sys.exit(f"{store}: the template has no row for {missing}")
    with out.open("w", encoding="utf-8-sig", newline="") as file:
        writer = csv.writer(file)
        writer.writerow(head[:3] + ["default"] + [store for _, store in LANGUAGES])
        for field, number, kind, *_ in body:
            writer.writerow([field, number, kind, SHARED.get(field, "")] + [listings[store].get(field, "") for _, store in LANGUAGES])
    print(f"{out}: {len(body)} rows, {len(LANGUAGES)} languages")


if __name__ == "__main__":
    main()
