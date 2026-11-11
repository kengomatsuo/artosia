"""Writes a language's phone texts into the remote's page.

    python -I tools/apply-phone-table.py <table.json> [code]

The page (`crates/artosia-remote/src/page.html`) carries English and Indonesian in its own
`STR` object, because it must work the moment it loads. A table is `{key: {"text": ...}}`
with the keys of `STR.en` and `label.<kind>` for the six section names. This replaces the
block of one language with the table's texts, in the order of the English block, and keeps
the page's own text for a key the table lacks.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE = os.path.join(ROOT, "crates", "artosia-remote", "src", "page.html")


def block(page, code):
    start = page.index(f"  {code}: {{\n")
    end = page.index("\n  }", start) + len("\n  }")
    return start, end


def main():
    table = json.load(open(sys.argv[1], encoding="utf-8"))
    code = sys.argv[2] if len(sys.argv) > 2 else "id"
    page = open(PAGE, encoding="utf-8").read()
    en_start, en_end = block(page, "en")
    english = dict(re.findall(r'(\w+): ("(?:[^"\\]|\\.)*")', page[en_start:en_end]))
    start, end = block(page, code)
    old = dict(re.findall(r'(\w+): ("(?:[^"\\]|\\.)*")', page[start:end]))
    lines, kept = [], []
    for key in english:
        if key in table:
            value = json.dumps(table[key]["text"], ensure_ascii=False)
        else:
            value = old.get(key, english[key])
            kept.append(key)
        lines.append(f"    {key}: {value}")
    page = page[:start] + f"  {code}: {{\n" + ",\n".join(lines) + "\n  }" + page[end:]
    if code == "id":
        name = lambda kind, fallback: table.get(f"label.{kind}", {}).get("text", fallback)
        labels = (
            f'  [/^Verse (\\d+)$/i, "{name("verse", "Bait")} $1"], [/^Chorus$/i, "{name("chorus", "Reff")}"], [/^Pre-?chorus$/i, "{name("pre_chorus", "Pra-reff")}"],\n'
            f'  [/^Bridge( \\d+)?$/i, "{name("bridge", "Bridge")}$1"], [/^Intro$/i, "{name("intro", "Intro")}"], [/^(Ending|Outro)$/i, "{name("ending", "Ending")}"]\n'
        )
        a = page.index("const LABELS_ID = [\n") + len("const LABELS_ID = [\n")
        b = page.index("];", a)
        page = page[:a] + labels + page[b:]
    open(PAGE, "w", encoding="utf-8", newline="\n").write(page)
    print(len(english) - len(kept), "texts written; kept as they were:", kept)


if __name__ == "__main__":
    main()
