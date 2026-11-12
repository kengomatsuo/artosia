"""Checks every homepage language file against the brief: the same keys as English, the
brief's tokens and link markers kept, the visible text within the brief's length, and none of
the marks the house bans from a string a person reads.

    python -I tools/check-site-strings.py [key ...]
"""
import json
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1] / "site" / "strings"
brief = json.loads((root / "brief.json").read_text(encoding="utf-8"))
english = json.loads((root / "en.json").read_text(encoding="utf-8"))
codes = ["en", "id", "zh-Hans", "zh-Hant", "ko", "ja", "es", "pt-BR", "fr", "vi"]
marker = re.compile(r"\{/?[a-z]+\}")
banned = {"—": "em dash", "–": "en dash", "·": "middle dot", "!": "exclamation", "！": "exclamation"}
sys.stdout.reconfigure(encoding="utf-8")
only = set(sys.argv[1:])
problems = 0
for code in codes:
    table = json.loads((root / f"{code}.json").read_text(encoding="utf-8"))
    lines = []
    missing = [k for k in english if k not in table]
    extra = [k for k in table if k not in english]
    if missing:
        lines.append(f"  missing: {missing}")
    if extra:
        lines.append(f"  extra: {extra}")
    for key, text in table.items():
        if only and key not in only:
            continue
        rule = brief.get(key, {})
        want = sorted(marker.findall(english.get(key, "")))
        have = sorted(marker.findall(text))
        if want != have:
            lines.append(f"  {key}: markers {have} want {want}")
        # A link's markers are not words a reader sees.
        visible = marker.sub("", text) if key.startswith("also_") else text
        limit = rule.get("max")
        if limit and len(visible) > limit:
            lines.append(f"  {key}: {len(visible)} characters, max {limit}: {visible}")
        for mark, name in banned.items():
            if mark in text:
                lines.append(f"  {key}: {name}: {text}")
    problems += len(lines)
    print(f"{code}: {len(table)} keys, {len(lines)} problems")
    if lines:
        print("\n".join(lines))
print(f"total problems: {problems}")
sys.exit(1 if problems else 0)
