"""Packs a MyBible module (SQLite) into the gzipped TSV Artosia embeds.

    python tools/make-bible-asset.py <module.SQLite3> crates/artosia-core/assets/kjv.tsv.gz

Only public-domain translations may be packed into the program. Lines are
book<TAB>chapter<TAB>verse<TAB>text with books numbered 1..66 in canonical order.
"""
import gzip
import re
import sqlite3
import sys

MYBIBLE = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 190, 220, 230, 240, 250, 260,
           290, 300, 310, 330, 340, 350, 360, 370, 380, 390, 400, 410, 420, 430, 440, 450, 460, 470, 480, 490, 500,
           510, 520, 530, 540, 550, 560, 570, 580, 590, 600, 610, 620, 630, 640, 650, 660, 670, 680, 690, 700, 710,
           720, 730]


def clean(text):
    text = re.sub(r"<[^>]*>", "", text).replace("¶", "").strip()
    # Some modules were built from CSV and kept its quoting.
    if len(text) >= 2 and text.startswith('"') and text.endswith('"'):
        text = text[1:-1].replace('""', '"')
    return " ".join(text.split())


def main():
    source, target = sys.argv[1], sys.argv[2]
    db = sqlite3.connect(source)
    rows = db.execute("select book_number, chapter, verse, text from verses order by book_number, chapter, verse")
    lines = []
    for number, chapter, verse, text in rows:
        if int(number) not in MYBIBLE:
            continue
        body = clean(text)
        if body:
            lines.append(f"{MYBIBLE.index(int(number)) + 1}\t{int(chapter)}\t{int(verse)}\t{body}")
    with gzip.GzipFile(target, "wb", compresslevel=9, mtime=0) as out:
        out.write(("\n".join(lines) + "\n").encode("utf-8"))
    print(f"{len(lines)} verses")


main()
