"""Checks, by their rectangles, that the plan's drop-down is whole on every side.

    cargo build -p artosia
    python tools/check-overlays.py

It asks the built window for its element tree (`--tree`) in each scene that opens the list,
in two sizes and both languages, and fails when the box leaves the plan column, when the list
does not start under the box or is narrower than it, or when the list comes nearer than 8
points to the window's right or bottom edge. The drop-down was once cut on the right because
it measured itself against a width the column did not have; this keeps that from coming back
unseen (docs/operator.md, "Every plan is in view under the plan").
"""
import json
import os
import subprocess
import sys
import tempfile

root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
exe = os.environ.get("ARTOSIA_EXE", os.path.join(root, "target", "debug", "artosia.exe" if os.name == "nt" else "artosia"))
KEYS = ("window", "plan", "plan-switcher-button", "plan-switcher-menu")


def rects(scene, size, flags):
    with tempfile.TemporaryDirectory() as folder:
        out = os.path.join(folder, "tree.json")
        subprocess.run([exe, "--scene", scene, "--size", size, "--tree", out, *flags], capture_output=True, timeout=300, check=False)
        tree = json.load(open(out, encoding="utf-8"))
    found = {}

    def walk(node):
        if isinstance(node, dict):
            key = node.get("key")
            if key in KEYS and node.get("rect") and key not in found:
                found[key] = node["rect"]
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(tree)
    return found


problems = 0
for scene in ("plans-drop", "plans-drop-long", "plans-drop-many"):
    for size in ("1366x768", "900x520"):
        for flags in ([], ["--indonesian"]):
            found = rects(scene, size, flags)
            name = f"{scene} {size} {' '.join(flags) or 'english'}"
            missing = [key for key in KEYS if key not in found]
            if missing:
                print(f"FAIL {name}: no {', '.join(missing)}")
                problems += 1
                continue
            window, plan, box, menu = (found[key] for key in KEYS)
            right = lambda r: r[0] + r[2]
            bottom = lambda r: r[1] + r[3]
            wrong = []
            if box[0] < plan[0] + 1 or right(box) > right(plan) - 1:
                wrong.append(f"the box {box} leaves the column {plan}")
            if abs(menu[0] - box[0]) > 0.5 or menu[2] < box[2] - 0.5:
                wrong.append(f"the list {menu} does not sit under the box {box}")
            if right(menu) > right(window) - 8 or bottom(menu) > bottom(window) - 8 or menu[0] < 0:
                wrong.append(f"the list {menu} leaves the window {window}")
            print(("FAIL " if wrong else "ok   ") + name + ("" if not wrong else ": " + "; ".join(wrong)))
            problems += len(wrong)
sys.exit(1 if problems else 0)
