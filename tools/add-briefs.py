"""Adds or replaces brief entries in docs/i18n/brief.json.

    python -I tools/add-briefs.py . <entries.json>

Each entry carries the brief's fields plus "after": the key it follows (for a new entry).
An entry whose key exists replaces it in place.
"""
import json
import sys

REGISTER = "The program speaks to a church volunteer at the console: any age, in the middle of a service, with no time to read. Plain, short, polite and familiar, the way one volunteer talks to another. No honorific for the reader, no jargon, no slang."

repo, path = sys.argv[1], sys.argv[2]
brief_path = repo + "/docs/i18n/brief.json"
briefs = json.load(open(brief_path, encoding="utf8"))
entries = json.load(open(path, encoding="utf8"))
for entry in entries:
    after = entry.pop("after", None)
    full = {"key": entry["key"], "kind": entry.get("kind", "const"), "screen": entry["screen"], "moment": entry["moment"], "role": entry["role"], "does": entry["does"], "budget": entry["budget"], "variables": entry.get("variables", []), "cases": entry.get("cases", []), "counts": entry.get("counts", ""), "register": entry.get("register", REGISTER), "concepts": entry.get("concepts", []), "notes": entry.get("notes", "")}
    keys = [b["key"] for b in briefs]
    if full["key"] in keys:
        briefs[keys.index(full["key"])] = full
    else:
        at = keys.index(after) + 1 if after in keys else len(briefs)
        briefs.insert(at, full)
json.dump(briefs, open(brief_path, "w", encoding="utf8"), ensure_ascii=False, indent=1)
open(brief_path, "a", encoding="utf8").write("\n")
print(len(briefs), "briefs")
