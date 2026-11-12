"""Writes the sentences macOS shows in its permission dialogs into a Mac app, one folder per
language: <Resources>/<code>.lproj/InfoPlist.strings, from tools/macos/strings/<code>.json.

    python3 tools/macos/info-strings.py <app>/Contents/Resources

English is in Info.plist itself, where macOS looks when a language has no file. Every other
language is written from tools/macos/strings/brief.json by its own writer who never sees the
English. A language that lacks a key, or has one more, stops the build.
"""
import json
import plistlib
import sys
from pathlib import Path

strings = Path(__file__).resolve().parent / "strings"
resources = Path(sys.argv[1])
brief = json.loads((strings / "brief.json").read_text(encoding="utf-8"))
keys = [key for key in brief if not key.startswith("_")]
english = json.loads((strings / "en.json").read_text(encoding="utf-8"))
if list(english) != keys:
    sys.exit(f"en.json holds {list(english)}, the brief {keys}")
written = []
for path in sorted(strings.glob("*.json")):
    code = path.stem
    if code in ("brief", "en"):
        continue
    table = json.loads(path.read_text(encoding="utf-8"))
    if list(table) != keys:
        sys.exit(f"{path.name} holds {list(table)}, the brief {keys}")
    for key, text in table.items():
        limit = brief[key]["max"]
        if len(text) > limit:
            sys.exit(f"{path.name}: {key} is {len(text)} characters, at most {limit}")
    folder = resources / f"{code}.lproj"
    folder.mkdir(parents=True, exist_ok=True)
    # A strings file may be a property list, which needs no escaping of quotes or accents.
    with (folder / "InfoPlist.strings").open("wb") as out:
        plistlib.dump(table, out, sort_keys=False)
    written.append(code)
print(f"permission sentences: {', '.join(written)}")
