"""Checks docs/i18n/brief.json and docs/i18n/brief-phone.json against the code.

    python -I tools/check-brief.py

It lists every key the operator window reads from `crates/artosia/src/strings.rs` and
`languages.rs`, and fails when an entry is missing, extra, over a sane budget, has a field
left empty, or when a `does` shares four words in a row with the English text of its key.
Names of keys on the keyboard and proper nouns are left out on purpose: LEFT_OUT below says
why, and docs/i18n/README.md repeats it.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRINGS = os.path.join(ROOT, "crates", "artosia", "src", "strings.rs")
PHONE = os.path.join(ROOT, "crates", "artosia-remote", "src", "page.html")

# Never translated, so never briefed.
LEFT_OUT = {
    "APP_NAME": "the app's name, a proper noun",
    "KEY_SEARCH": "a key as the keyboard prints it",
    "KEY_ENTER": "a key as the keyboard prints it",
    "KEY_SHOW_NOW": "keys as the keyboard prints them",
    "KEY_ADD": "keys as the keyboard prints them",
    "KEY_SAVE": "keys as the keyboard prints them",
    "KEY_START_HERE": "keys as the keyboard prints them",
    "ctrl_digit": "a key as the keyboard prints it",
    "STREAM_ADDRESS_HINT": "a sample web address",
    "BIBLES_SOURCE_EBIBLE": "the name of a website",
    "BIBLES_SOURCE_BEBLIA": "the name of a website",
    "language_code": "not a text",
    "MONTHS": "briefed month by month as MONTHS[n]",
    "ANNOUNCEMENT_TEMPLATES": "briefed row by row",
    "KEYS_SHOWING_KEYS": "the key column, drawn as the keyboard prints it",
    "KEYS_FINDING_KEYS": "the key column, drawn as the keyboard prints it",
    "KEYS_PLAN_KEYS": "the key column, drawn as the keyboard prints it",
    "KEYS_EDITING_KEYS": "the key column, drawn as the keyboard prints it",
    "KEYS_SHOWING_ROWS": "briefed row by row",
    "KEYS_FINDING_ROWS": "briefed row by row",
    "KEYS_PLAN_ROWS": "briefed row by row",
    "KEYS_EDITING_ROWS": "briefed row by row",
    "section_label": "briefed through section.* and section_numbered",
}
KINDS = {"const", "sentence", "row", "section", "noun", "months", "phone"}
FIELDS = ["key", "kind", "screen", "moment", "role", "does", "budget", "variables", "cases", "counts", "register", "concepts", "notes"]
MIN_DOES, MAX_BUDGET = 12, 170


def words(text):
    return re.findall(r"[a-z0-9']+", text.lower())


def grams(text, n=4):
    w = words(text)
    return {tuple(w[i:i + n]) for i in range(len(w) - n + 1)}


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def code_of(source):
    return source.split("#[cfg(test)]\nmod tests")[0]


def function_bodies(code):
    """name -> body text of each top-level `pub fn`."""
    out = {}
    starts = [(m.start(), m.group(1)) for m in re.finditer(r"^pub fn (\w+)", code, re.M)]
    for i, (at, name) in enumerate(starts):
        end = starts[i + 1][0] if i + 1 < len(starts) else len(code)
        out[name] = code[at:end]
    return out


def literals(text):
    return re.findall(r'"((?:[^"\\]|\\.)*)"', text)


def expected_operator():
    code = code_of(read(STRINGS))
    english = {}
    for m in re.finditer(r'^text!\((\w+), "((?:[^"\\]|\\.)*)"', code, re.M):
        if m.group(1) not in LEFT_OUT:
            english[m.group(1)] = [m.group(2)]
    for name, body in function_bodies(code).items():
        if name in LEFT_OUT or name in english:
            continue
        if re.match(r"pub fn \w+\([^{]*?\)\s*->\s*String", body) is None:
            continue
        english[name] = literals(body)
    # arrays: second column of the key sheet, both columns of the templates
    for name in ("SHOWING", "FINDING", "PLAN", "EDITING"):
        m = re.search(r"const KEYS_%s_ROWS_EN: \[\(&str, &str\); (\d+)\] = \[(.*?)\n\];" % name, code, re.S)
        rows = re.findall(r'\("((?:[^"\\]|\\.)*)", "((?:[^"\\]|\\.)*)"\)', m.group(2))
        assert len(rows) == int(m.group(1)), name
        for i, (_, second) in enumerate(rows):
            english["KEYS_%s_ROWS[%d].1" % (name, i)] = [second]
    m = re.search(r"const ANNOUNCEMENT_TEMPLATES_EN: \[\(&str, &str\); (\d+)\] = \[(.*?)\n\];", code, re.S)
    rows = re.findall(r'\("((?:[^"\\]|\\.)*)", "((?:[^"\\]|\\.)*)"\)', m.group(2))
    for i, (first, second) in enumerate(rows):
        english["ANNOUNCEMENT_TEMPLATES[%d].0" % i] = [first]
        english["ANNOUNCEMENT_TEMPLATES[%d].1" % i] = [second]
    m = re.search(r"const MONTHS_EN: \[&str; 12\] = \[(.*?)\];", code, re.S)
    for i, month in enumerate(literals(m.group(1))):
        english["MONTHS[%d]" % i] = [month]
    # sections: the kinds section_label_table reads, and the counted nouns
    m = re.search(r"fn section_label_table.*?\n}\n", code, re.S)
    for kind, label in re.findall(r'=> \("(\w+)", "([^"]+)"\)', m.group(0)):
        english["section.%s" % kind] = [label]
    english["section_numbered"] = []
    for noun in sorted(set(re.findall(r'counted\("(\w+)"', code))):
        english["noun.%s" % noun] = []
    return english


def expected_phone():
    html = read(PHONE)
    block = re.search(r"const STR = \{\s*en: \{(.*?)\n  \},\n  id:", html, re.S).group(1)
    keys = {}
    for m in re.finditer(r'(\w+): "((?:[^"\\]|\\.)*)"', block):
        keys[m.group(1)] = [m.group(2)]
    for kind in ("verse", "chorus", "pre_chorus", "bridge", "intro", "ending"):
        keys["label." + kind] = []
    return keys


def check(path, expected, problems, phone=False):
    try:
        entries = json.loads(read(path))
    except (OSError, ValueError) as err:
        problems.append("%s: cannot read: %s" % (path, err))
        return 0
    seen = {}
    for entry in entries:
        key = entry.get("key", "?")
        if key in seen:
            problems.append("%s: duplicate entry" % key)
        seen[key] = entry
        for field in FIELDS:
            if field not in entry:
                problems.append("%s: no field %s" % (key, field))
        if entry.get("kind") not in KINDS:
            problems.append("%s: unknown kind %r" % (key, entry.get("kind")))
        if phone != (entry.get("kind") == "phone"):
            problems.append("%s: wrong file for kind %s" % (key, entry.get("kind")))
        for field in ("screen", "moment", "role", "does", "register"):
            if len(str(entry.get(field, "")).strip()) < (MIN_DOES if field == "does" else 3):
                problems.append("%s: %s is empty or too short" % (key, field))
        budget = entry.get("budget")
        if not isinstance(budget, int) or not 1 <= budget <= MAX_BUDGET:
            problems.append("%s: budget %r is not a number between 1 and %d" % (key, budget, MAX_BUDGET))
        if not isinstance(entry.get("concepts"), list):
            problems.append("%s: concepts is not a list" % key)
        english = expected.get(key)
        if english is not None:
            mine = grams(entry.get("does", ""))
            for text in english:
                shared = mine & grams(text)
                if shared:
                    problems.append("%s: does shares four words with the English: %s" % (key, " ".join(sorted(shared)[0])))
        if entry.get("kind") == "sentence":
            has_vars = bool(entry.get("variables")) or bool(entry.get("counts"))
            if not has_vars and key not in ("update_none",):
                problems.append("%s: a sentence with no variable and no count" % key)
            for var in entry.get("variables", []):
                if not var.get("example") or not var.get("name"):
                    problems.append("%s: a variable lacks a name or an example" % key)
    for key in sorted(set(expected) - set(seen)):
        problems.append("%s: missing" % key)
    for key in sorted(set(seen) - set(expected)):
        problems.append("%s: extra, no such key in the code" % key)
    return len(entries)


def concept_ids():
    return set(re.findall(r"`([a-z_]+)`", read(os.path.join(ROOT, "docs", "i18n", "concepts.md"))))


def main():
    problems = []
    got = check(os.path.join(ROOT, "docs", "i18n", "brief.json"), expected_operator(), problems)
    got += check(os.path.join(ROOT, "docs", "i18n", "brief-phone.json"), expected_phone(), problems, phone=True)
    known = concept_ids()
    for name in ("brief.json", "brief-phone.json"):
        for entry in json.loads(read(os.path.join(ROOT, "docs", "i18n", name))):
            for concept in entry.get("concepts", []):
                if concept not in known:
                    problems.append("%s: concept %s is not described in concepts.md" % (entry["key"], concept))
    for problem in problems:
        print(problem)
    print("%d entries, %d problems" % (got, len(problems)))
    sys.exit(1 if problems else 0)


main()
