"""Builds the homepage into target/site: one template, one page per language.

    python -I tools/build-site.py [--version V] [--setup-bytes N] [--zip-bytes N] [--langs en,id] [--out target/site]

The version comes from --version, or from crates/artosia/Cargo.toml. The sizes
of the setup and of the zip come from --setup-bytes and --zip-bytes, or from
the files in dist/<version>/ when that was built here. tools/publish-site.sh
passes all three from the newest release.
A placeholder with no string, a string no placeholder uses, and a language
missing a key each stop the build.
"""

import argparse
import html
import json
import re
import shutil
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
DOMAIN = "artosia.matsuokengo.com"
ORIGIN = f"https://{DOMAIN}"
LATEST = "https://github.com/kengomatsuo/artosia/releases/latest/download"
DOWNLOAD = f"{LATEST}/Artosia-setup.exe"
ZIP = f"{LATEST}/Artosia-windows.zip"
# The first page sends an Indonesian browser to /id/ once; a chosen language is kept.
REDIRECT = (
    '<script>try{var c=localStorage.getItem("lang");'
    'if(c==="id"||(!c&&/^id\\b/i.test(navigator.language||"")))location.replace("id/")}'
    "catch(e){}</script>"
)
PLACE = re.compile(r"\{\{([a-z0-9_:/.\-]+)\}\}")


def version() -> str:
    text = (ROOT / "crates/artosia/Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version = "([^"]+)"', text, re.M).group(1)


def png_size(path: Path) -> tuple[int, int]:
    with path.open("rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit(f"{path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def page(lang: str, strings: dict, numbers: dict) -> str:
    first = lang == "en"
    fixed = {
        "lang": lang,
        "root": "" if first else "../",
        "home": "./",
        "other_lang": "id" if first else "en",
        "other_href": "id/" if first else "../",
        "site": ORIGIN,
        "canonical": f"{ORIGIN}/" if first else f"{ORIGIN}/{lang}/",
        "download": DOWNLOAD,
        "zip": ZIP,
        "version": numbers["version"],
        "language_redirect": REDIRECT if first else "",
    }
    used = set()

    def words(key: str) -> str:
        used.add(key)
        text = html.escape(strings[key], quote=True)
        text = text.replace("{version}", f'<span data-version>{numbers["version"]}</span>')
        text = text.replace("{zipsize}", f'<span data-zip-size>{numbers["zip_mb"]}</span>')
        return text.replace("{size}", f'<span data-size>{numbers["mb"]}</span>')

    def fill(match: re.Match) -> str:
        key = match.group(1)
        if key in fixed:
            return fixed[key]
        if key.startswith("wh:"):
            w, h = png_size(SITE / key[3:])
            return f'width="{w}" height="{h}"'
        if key.startswith("json:"):
            used.add(key[5:])
            return json.dumps(strings[key[5:]], ensure_ascii=False)[1:-1]
        if key not in strings:
            sys.exit(f"page.html asks for {{{{{key}}}}}, which {lang}.json does not have")
        return words(key)

    out = PLACE.sub(fill, (SITE / "page.html").read_text(encoding="utf-8"))
    unused = sorted(set(strings) - used)
    if unused:
        sys.exit(f"{lang}.json has strings the page never shows: {', '.join(unused)}")
    return out


def main() -> None:
    args = argparse.ArgumentParser()
    args.add_argument("--version")
    args.add_argument("--setup-bytes", type=int)
    args.add_argument("--zip-bytes", type=int)
    args.add_argument("--langs", default="en,id")
    args.add_argument("--out", default=str(ROOT / "target/site"))
    args = args.parse_args()

    v = args.version or version()
    def size_of(given: int | None, name: str, flag: str) -> int:
        if given is not None:
            return given
        built = ROOT / "dist" / v / name
        if not built.exists():
            sys.exit(f"pass {flag}: {built} is not here")
        return built.stat().st_size

    def megabytes(n: int) -> int:
        return max(1, round(n / 1048576))

    numbers = {
        "version": v,
        "mb": megabytes(size_of(args.setup_bytes, "Artosia-setup.exe", "--setup-bytes")),
        "zip_mb": megabytes(size_of(args.zip_bytes, "Artosia-windows.zip", "--zip-bytes")),
    }

    langs = args.langs.split(",")
    catalogues = {
        lang: json.loads((SITE / "strings" / f"{lang}.json").read_text(encoding="utf-8"))
        for lang in langs
    }
    for lang in langs[1:]:
        odd = set(catalogues[lang]) ^ set(catalogues[langs[0]])
        if odd:
            sys.exit(f"{lang}.json and {langs[0]}.json differ in: {', '.join(sorted(odd))}")

    out = Path(args.out)
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    for lang in langs:
        folder = out if lang == "en" else out / lang
        folder.mkdir(exist_ok=True)
        (folder / "index.html").write_text(
            page(lang, catalogues[lang], numbers), encoding="utf-8", newline="\n"
        )
    for name in ("style.css", "script.js"):
        shutil.copy(SITE / name, out / name)
    for name in ("fonts", "img"):
        shutil.copytree(SITE / name, out / name)
    shutil.copy(ROOT / "assets/brand/artosia.ico", out / "favicon.ico")
    shutil.copy(ROOT / "assets/brand/artosia-icon.svg", out / "img" / "icon.svg")

    (out / "CNAME").write_text(DOMAIN + "\n", newline="\n")
    (out / ".nojekyll").write_text("")
    (out / "robots.txt").write_text(
        f"User-agent: *\nAllow: /\n\nSitemap: {ORIGIN}/sitemap.xml\n", newline="\n"
    )
    urls = "".join(
        f"  <url><loc>{ORIGIN}/{'' if lang == 'en' else lang + '/'}</loc></url>\n"
        for lang in langs
    )
    (out / "sitemap.xml").write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        f"{urls}</urlset>\n",
        newline="\n",
    )
    print(
        f"built {out}: {', '.join(langs)}, version {v}, "
        f"setup {numbers['mb']} MB, zip {numbers['zip_mb']} MB"
    )


if __name__ == "__main__":
    main()
