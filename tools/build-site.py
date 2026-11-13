"""Builds the homepage into target/site: one template, one page per language.

    python -I tools/build-site.py [--version V] [--setup-bytes N] [--zip-bytes N]
        [--linux-bytes N] [--mac-bytes N] [--linux-version V] [--mac-version V]
        [--langs en,id] [--out target/site]

The version comes from --version, or from crates/artosia/Cargo.toml. The sizes
of the setup, the zip and the Linux package come from --setup-bytes, --zip-bytes
and --linux-bytes, or from the files in dist/<version>/ when that was built here.
The Mac row, its link and the Mac words of the hero exist only when --mac-bytes
is given or dist/<version>/Artosia-macos-aarch64.dmg is here: a system shows only
when its download exists. tools/publish-site.sh passes the sizes from the newest release.
A placeholder with no string, a string no placeholder uses, and a language
missing a key each stop the build; both variants (with and without the Mac)
are checked on every build.
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
# The setup is in every release. Another system's file is in a release only when its build
# worked, so its link names the release the page was built for and cannot go stale.
RELEASE = "https://github.com/kengomatsuo/artosia/releases/download"
# Every language the page is written in, English first, each under its own name. The program
# speaks the same ten (`crates/artosia/src/languages.rs`).
LANGUAGES = [
    ("en", "English"),
    ("id", "Bahasa Indonesia"),
    ("zh-Hans", "\u7b80\u4f53\u4e2d\u6587"),
    ("zh-Hant", "\u7e41\u9ad4\u4e2d\u6587"),
    ("ko", "\ud55c\uad6d\uc5b4"),
    ("ja", "\u65e5\u672c\u8a9e"),
    ("es", "Espa\u00f1ol"),
    ("pt-BR", "Portugu\u00eas (Brasil)"),
    ("fr", "Fran\u00e7ais"),
    ("vi", "Ti\u1ebfng Vi\u1ec7t"),
]
# The first page sends a browser to its own language once; a chosen language is kept. The
# rule that fits a browser's tag to a page is the one of crates/artosia-locale: the same code,
# then Chinese by its script, then the same first part.
REDIRECT = (
    "<script>try{var L=%s,c=localStorage.getItem(\"lang\"),p=null,"
    "f=function(t){t=String(t||\"\").replace(/_/g,\"-\").toLowerCase();var k=[\"en\"].concat(L),i;"
    "for(i=0;i<k.length;i++)if(k[i].toLowerCase()===t)return k[i];"
    "var s=t.split(\"-\"),b=s[0]===\"in\"?\"id\":s[0];"
    "if(b===\"zh\"){var r=s.slice(1),h=function(l){for(var j=0;j<r.length;j++)if(l.indexOf(r[j])>=0)return true;return false},"
    "z=!h([\"hans\"])&&h([\"hant\",\"tw\",\"hk\",\"mo\"])?\"zh-hant\":\"zh-hans\";"
    "for(i=0;i<k.length;i++)if(k[i].toLowerCase()===z)return k[i]}"
    "for(i=0;i<k.length;i++)if(k[i].toLowerCase().split(\"-\")[0]===b)return k[i];return null};"
    "if(c){if(L.indexOf(c)>=0)p=c}else{var a=navigator.languages&&navigator.languages.length?navigator.languages:[navigator.language||\"\"];"
    "for(var n=0;n<a.length;n++){var x=f(a[n]);if(x){if(x!==\"en\")p=x;break}}}"
    "if(p)location.replace(p+\"/\")}catch(e){}</script>"
)
PLACE = re.compile(r"\{\{([a-z0-9_:/.\-]+)\}\}")
# <!--if mac-->...<!--/if--> stays only when the Mac download exists, <!--if nomac--> only when not.
VARIANT = re.compile(r"<!--if (mac|nomac)-->(.*?)<!--/if-->", re.S)


def version() -> str:
    text = (ROOT / "crates/artosia/Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version = "([^"]+)"', text, re.M).group(1)


def png_size(path: Path) -> tuple[int, int]:
    with path.open("rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit(f"{path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def address(lang: str) -> str:
    return f"{ORIGIN}/" if lang == "en" else f"{ORIGIN}/{lang}/"


def page(lang: str, strings: dict, numbers: dict, langs: list[str]) -> str:
    first = lang == "en"
    root = "" if first else "../"
    names = dict(LANGUAGES)
    links = "".join(
        f'<li><a href="{root}{"" if code == "en" else code + "/"}" hreflang="{code}" lang="{code}" data-lang="{code}"'
        f'{" aria-current=\"true\"" if code == lang else ""}>{html.escape(names[code])}</a></li>'
        for code in langs
    )
    alternates = "".join(f'<link rel="alternate" hreflang="{code}" href="{address(code)}">\n' for code in langs)
    fixed = {
        "lang": lang,
        "root": root,
        "home": "./",
        # A language whose pictures are not rendered yet shows the English window.
        "img_lang": lang if (SITE / "img" / lang).is_dir() else "en",
        "language_name": html.escape(names[lang]),
        "language_links": links,
        "alternates": alternates + f'<link rel="alternate" hreflang="x-default" href="{ORIGIN}/">',
        "in_language": json.dumps(langs),
        "privacy": f"{root}privacy/",
        "site": ORIGIN,
        "canonical": address(lang),
        "download": DOWNLOAD,
        "zip": ZIP,
        "linux": f"{RELEASE}/v{numbers['linux_version']}/Artosia-linux-x86_64.tar.gz",
        "mac": f"{RELEASE}/v{numbers['mac_version']}/Artosia-macos-aarch64.dmg",
        "version": numbers["version"],
        "language_redirect": REDIRECT % json.dumps([code for code in langs if code != "en"]) if first else "",
    }
    template = (SITE / "page.html").read_text(encoding="utf-8")
    # Both variants are checked, so the build with the Mac and the one without it can never drift.
    asked = {m.group(1) for m in PLACE.finditer(template)}
    asked = {k[5:] if k.startswith("json:") else k for k in asked}
    asked = {k for k in asked if k not in fixed and not k.startswith(("wh:", "icon:"))}
    missing = sorted(asked - set(strings))
    if missing:
        sys.exit(f"page.html asks for {', '.join(missing)}, which {lang}.json does not have")
    unused = sorted(set(strings) - asked)
    if unused:
        sys.exit(f"{lang}.json has strings the page never shows: {', '.join(unused)}")
    template = VARIANT.sub(lambda m: m.group(2) if (m.group(1) == "mac") == ("mac_mb" in numbers) else "", template)

    def words(key: str) -> str:
        text = html.escape(strings[key], quote=True)
        text = text.replace("{version}", f'<span data-version>{numbers["version"]}</span>')
        text = text.replace("{zipsize}", f'<span data-zip-size>{numbers["zip_mb"]}</span>')
        text = text.replace("{linuxsize}", f'<span data-linux-size>{numbers["linux_mb"]}</span>')
        text = text.replace("{macsize}", f'<span data-mac-size>{numbers.get("mac_mb", "")}</span>')
        # A sentence with links is one template; the writer places each link around its own word.
        for system in ("windows", "linux", "mac"):
            text = text.replace("{%s}" % system, f'<a href="#{system}">').replace("{/%s}" % system, "</a>")
        return text.replace("{size}", f'<span data-size>{numbers["mb"]}</span>')

    def fill(match: re.Match) -> str:
        key = match.group(1)
        if key in fixed:
            return fixed[key]
        if key.startswith("wh:"):
            w, h = png_size(SITE / key[3:])
            return f'width="{w}" height="{h}"'
        if key.startswith("icon:"):
            # A system's mark, drawn in the colour of the words beside it. The file keeps its
            # licence comment, which is the attribution its licence asks for.
            mark = (SITE / "img" / "os" / f"{key[5:]}.svg").read_text(encoding="utf-8").strip()
            return mark.replace("<svg ", '<svg class="os-icon" aria-hidden="true" focusable="false" ', 1)
        if key.startswith("json:"):
            return json.dumps(strings[key[5:]], ensure_ascii=False)[1:-1]
        return words(key)

    return PLACE.sub(fill, template)


def privacy() -> str:
    """The privacy page: English only, because a policy is not written by translation."""
    text = json.loads((SITE / "privacy.json").read_text(encoding="utf-8"))
    sections = "".join(
        f"<h2>{html.escape(section['heading'])}</h2>\n"
        + "".join(f"<p>{html.escape(paragraph)}</p>\n" for paragraph in section["paragraphs"])
        for section in text["sections"]
    )
    fixed = {
        "title": html.escape(text["title"]),
        "lead": html.escape(text["lead"]),
        "updated": html.escape(text["updated"]),
        "sections": sections,
        "contact": html.escape(text["contact"]),
        "site": ORIGIN,
    }
    return PLACE.sub(lambda match: fixed[match.group(1)], (SITE / "privacy.html").read_text(encoding="utf-8"))


def main() -> None:
    args = argparse.ArgumentParser()
    args.add_argument("--version")
    args.add_argument("--setup-bytes", type=int)
    args.add_argument("--zip-bytes", type=int)
    args.add_argument("--linux-bytes", type=int)
    args.add_argument("--mac-bytes", type=int)
    # The release each of these two files is taken from, when it is older than --version: a
    # release goes out without a system's file when that system's machine could not be reached.
    args.add_argument("--linux-version")
    args.add_argument("--mac-version")
    args.add_argument("--langs", default=",".join(code for code, _ in LANGUAGES))
    args.add_argument("--out", default=str(ROOT / "target/site"))
    args = args.parse_args()

    v = args.version or version()
    def size_of(given: int | None, name: str, flag: str, required: bool = True) -> int | None:
        if given is not None:
            return given
        built = ROOT / "dist" / v / name
        if not built.exists():
            if required:
                sys.exit(f"pass {flag}: {built} is not here")
            return None
        return built.stat().st_size

    def megabytes(n: int) -> int:
        return max(1, round(n / 1048576))

    numbers = {
        "version": v,
        "linux_version": args.linux_version or v,
        "mac_version": args.mac_version or v,
        "mb": megabytes(size_of(args.setup_bytes, "Artosia-setup.exe", "--setup-bytes")),
        "zip_mb": megabytes(size_of(args.zip_bytes, "Artosia-windows.zip", "--zip-bytes")),
        "linux_mb": megabytes(size_of(args.linux_bytes, "Artosia-linux-x86_64.tar.gz", "--linux-bytes")),
    }
    mac = size_of(args.mac_bytes, "Artosia-macos-aarch64.dmg", "--mac-bytes", required=False)
    if mac is not None:
        numbers["mac_mb"] = megabytes(mac)

    langs = args.langs.split(",")
    unknown = [lang for lang in langs if lang not in dict(LANGUAGES)]
    if unknown:
        sys.exit(f"no such language: {', '.join(unknown)}")
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
            page(lang, catalogues[lang], numbers, langs), encoding="utf-8", newline="\n"
        )
    (out / "privacy").mkdir()
    (out / "privacy" / "index.html").write_text(privacy(), encoding="utf-8", newline="\n")
    for name in ("style.css", "script.js"):
        shutil.copy(SITE / name, out / name)
    for name in ("fonts", "img"):
        shutil.copytree(SITE / name, out / name)
    shutil.copy(ROOT / "assets/brand/artosia.ico", out / "favicon.ico")
    shutil.copy(ROOT / "assets/brand/artosia-icon.svg", out / "img" / "icon.svg")

    (out / "robots.txt").write_text(
        f"User-agent: *\nAllow: /\n\nSitemap: {ORIGIN}/sitemap.xml\n", newline="\n"
    )
    urls = "".join(f"  <url><loc>{address(lang)}</loc></url>\n" for lang in langs)
    urls += f"  <url><loc>{ORIGIN}/privacy/</loc></url>\n"
    (out / "sitemap.xml").write_text(
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        f"{urls}</urlset>\n",
        newline="\n",
    )
    print(
        f"built {out}: {', '.join(langs)}, version {v}, "
        f"setup {numbers['mb']} MB, zip {numbers['zip_mb']} MB, linux {numbers['linux_mb']} MB, "
        f"mac {str(numbers['mac_mb']) + ' MB' if 'mac_mb' in numbers else 'none'}"
    )


if __name__ == "__main__":
    main()
