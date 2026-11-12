"""Writes crates/artosia-core/assets/beblia-names.json: the name each Bible of the Beblia
repository gives itself, and the short code people call it by.

    python -I tools/make-beblia-names.py [<folder to keep the fetched headers in>]

The repository's list (one call to GitHub's git-tree API) has file names only, and a file name
says little: `IndonesianBible.xml` is the Terjemahan Baru. Each file opens with
`<bible translation="..." status="..." link="...">`, so this reads the first 4,096 bytes of every
file (the same bytes the program reads when a row is picked) and keeps two things:

  name   the `translation` attribute as the file writes it (`language` or `name` in the files that use those)
  code   the last part of a bible.com link (`.../MAT.4.TB` gives `TB`), when the file has one

The headers are kept in the folder (default `~/Shared/research/artosia-bibles/beblia-headers`), so
a second run fetches only files it has not seen. The asset is `{"<file>": {"name": ..., "code": ...}}`
with `code` left out when there is none; a file with no readable name is left out and shows by
its file name, as every file did before 0.1.18.
"""
import html, json, os, re, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

TREE = "https://api.github.com/repos/Beblia/Holy-Bible-XML-Format/git/trees/master?recursive=1"
RAW = "https://raw.githubusercontent.com/Beblia/Holy-Bible-XML-Format/master/"
AGENT = {"User-Agent": "artosia-tools"}
ROOT = Path(__file__).resolve().parents[1]
folder = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(os.path.expanduser("~/Shared/research/artosia-bibles/beblia-headers"))
folder.mkdir(parents=True, exist_ok=True)


def get(url, first_bytes=None):
    headers = dict(AGENT)
    if first_bytes:
        headers["Range"] = "bytes=0-%d" % (first_bytes - 1)
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=60) as answer:
        return answer.read()


def head_of(file):
    kept = folder / (file[: -len(".xml")] + ".head")
    if not kept.exists() or kept.stat().st_size == 0:
        kept.write_bytes(get(RAW + file, 4096))
    return kept.read_bytes().decode("utf-8-sig", errors="replace")


def read(file):
    try:
        text = head_of(file)
    except Exception as error:
        return file, None, str(error)
    tag = re.search(r"<bible\b[^>]*>", text, re.S)
    if not tag:
        return file, None, "no <bible> tag in the first 4,096 bytes"
    attributes = {name: html.unescape(value).strip() for name, value in re.findall(r'(\w+)\s*=\s*"([^"]*)"', tag.group(0))}
    # Most files say `translation`; some say `language` or `name` for the same thing.
    name = " ".join((attributes.get("translation") or attributes.get("language") or attributes.get("name") or "").split())
    if not name:
        return file, None, "no translation, language or name attribute"
    entry = {"name": name}
    link = attributes.get("link") or attributes.get("site") or ""
    # bible.com writes a passage as BOOK.chapter.VERSION; the version is the code people know.
    code = re.search(r"bible\.com/(?:[a-z-]+/)?bible/\d+/[0-9A-Z]{3}\.\d+\.([^/?#\s]+)$", link)
    if code:
        entry["code"] = code.group(1)
    return file, entry, None


tree = json.loads(get(TREE).decode("utf-8"))
files = sorted(entry["path"] for entry in tree["tree"] if entry["type"] == "blob" and "/" not in entry["path"] and entry["path"].endswith("Bible.xml"))
with ThreadPoolExecutor(max_workers=12) as pool:
    results = list(pool.map(read, files))
names = {file: entry for file, entry, _ in results if entry}
for file, entry, problem in results:
    if not entry:
        print("left out:", file, "-", problem)
out = ROOT / "crates/artosia-core/assets/beblia-names.json"
with out.open("w", encoding="utf-8", newline="\n") as handle:
    json.dump(names, handle, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    handle.write("\n")
coded = sum(1 for entry in names.values() if "code" in entry)
print("%d files, %d named, %d with a code, %d bytes" % (len(files), len(names), coded, out.stat().st_size))
