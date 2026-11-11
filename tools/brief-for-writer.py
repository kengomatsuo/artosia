"""Lays the briefs out for a writer, who reads them and nothing else.

    python -I tools/brief-for-writer.py <out folder>

`docs/i18n/brief.json` repeats the register on every entry and is too long to read in one go.
This writes the same briefs as a few plain files in the order of the code, the register and
the rules once at the top of the first, and the phone page's briefs as the last file. It
never prints a string of the program: the writer must not see one.
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PER_FILE = 150


def lay(entry):
    lines = [f"### {entry['key']}  ({entry.get('role', '')}, at most {entry.get('budget', '?')} characters)"]
    for field in ("screen", "moment", "does"):
        if entry.get(field):
            lines.append(f"{field}: {entry[field]}")
    for variable in entry.get("variables") or []:
        meaning = variable.get("meaning") or variable.get("what") or ""
        lines.append(f"variable {{{variable['name']}}}: {meaning} Example: {variable.get('example', '')}")
    cases = entry.get("cases") or []
    if cases:
        lines.append("cases: " + "; ".join(case if isinstance(case, str) else json.dumps(case, ensure_ascii=False) for case in cases))
    if entry.get("counts"):
        lines.append(f"counts: {entry['counts']}")
    if entry.get("concepts"):
        lines.append("concepts: " + ", ".join(entry["concepts"]))
    if entry.get("notes"):
        lines.append(f"notes: {entry['notes']}")
    return "\n".join(lines)


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    briefs = json.load(open(os.path.join(ROOT, "docs", "i18n", "brief.json"), encoding="utf-8"))
    phone = json.load(open(os.path.join(ROOT, "docs", "i18n", "brief-phone.json"), encoding="utf-8"))
    register = briefs[0].get("register", "")
    parts = [briefs[at:at + PER_FILE] for at in range(0, len(briefs), PER_FILE)]
    for number, part in enumerate(parts, 1):
        head = f"# Briefs, part {number} of {len(parts) + 1}: the operator's window\n\n"
        if number == 1:
            head += f"Who speaks to whom, in every entry unless it says otherwise: {register}\n\n"
        with open(os.path.join(out, f"briefs-{number}.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(head + "\n\n".join(lay(entry) for entry in part) + "\n")
    with open(os.path.join(out, f"briefs-{len(parts) + 1}.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(f"# Briefs, part {len(parts) + 1} of {len(parts) + 1}: the page on a phone\n\nThese keys go in a file of their own, `id-phone.json`.\n\n" + "\n\n".join(lay(entry) for entry in phone) + "\n")
    print(len(briefs), "operator briefs in", len(parts), "files;", len(phone), "phone briefs")


if __name__ == "__main__":
    main()
