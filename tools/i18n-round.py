"""Writer packs and merges for a blind-translation round.

    python -I tools/i18n-round.py pack . <out.md> <key> [<key> ...]
        Lays out the briefs of the given keys for a writer; never prints a program string.
    python -I tools/i18n-round.py merge . <folder>
        Merges <folder>/<code>.json ({key: {text, back, why}}) into docs/i18n/<code>.json.
"""
import json
import os
import sys


def pack(repo, out, keys):
    briefs = json.load(open(os.path.join(repo, "docs/i18n/brief.json"), encoding="utf8"))
    by_key = {b["key"]: b for b in briefs}
    lines = ["# Briefs for this round", "", "Register for every text: " + briefs[0]["register"], ""]
    for key in keys:
        e = by_key[key]
        lines.append(f"### {key}  ({e.get('role', '')}, at most {e.get('budget', '?')} characters)")
        for field in ("screen", "moment", "does"):
            if e.get(field):
                lines.append(f"{field}: {e[field]}")
        for v in e.get("variables") or []:
            lines.append(f"variable {{{v['name']}}}: {v.get('meaning', '')} Example: {v.get('example', '')}")
        if e.get("cases"):
            lines.append("cases: " + "; ".join(c if isinstance(c, str) else json.dumps(c, ensure_ascii=False) for c in e["cases"]))
        if e.get("counts"):
            lines.append(f"counts: {e['counts']}")
        if e.get("concepts"):
            lines.append("concepts: " + ", ".join(e["concepts"]))
        if e.get("notes"):
            lines.append(f"notes: {e['notes']}")
        lines.append("")
    open(out, "w", encoding="utf8").write("\n".join(lines))
    print(out, len(keys), "briefs")


def merge(repo, folder):
    for name in sorted(os.listdir(folder)):
        if not name.endswith(".json"):
            continue
        code = name[:-5]
        target = os.path.join(repo, "docs/i18n", f"{code}.json")
        if not os.path.exists(target):
            print("no table for", code)
            continue
        table = json.load(open(target, encoding="utf8"))
        new = json.load(open(os.path.join(folder, name), encoding="utf8"))
        for key, value in new.items():
            table[key] = {"text": value["text"], "back": value.get("back", ""), "why": value.get("why", "")}
        raw = open(target, encoding="utf8").read()
        indent = 1 if raw.startswith("{\n \"") else 2
        json.dump(table, open(target, "w", encoding="utf8"), ensure_ascii=False, indent=indent)
        open(target, "a", encoding="utf8").write("\n")
        print(code, len(new), "texts")


if __name__ == "__main__":
    if sys.argv[1] == "pack":
        pack(sys.argv[2], sys.argv[3], sys.argv[4:])
    else:
        merge(sys.argv[2], sys.argv[3])
