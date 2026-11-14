"""Renders each feature's window and fails if its help mark or help row is not drawn.

    python -I tools/check-help.py

Every feature the guide teaches has one place a person asks about it: the help mark beside its
window's name or the help row at the top of its dialog, keyed `help-<feature>`. A window that
loses it, or a new feature that never got one, fails here. Needs a debug build; ARTOSIA_EXE names
another one, such as a build made from HEAD.
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = Path(os.environ.get("ARTOSIA_EXE", ROOT / "target" / "debug" / "artosia.exe"))
# The help key of each feature (`Feature::key` in guide/lessons.rs) and a scene that opens it.
PLACES = [
    ("help-plan", "start"),
    ("help-notices", "tab-notices"),
    ("help-calendar", "tab-calendar"),
    ("help-people", "tab-people"),
    ("help-giving", "tab-giving"),
    ("help-templates", "tab-templates"),
    ("help-look", "tab-look"),
    ("help-screens", "screens"),
    ("help-phone", "remote"),
    ("help-listen", "listen"),
    ("help-stream", "stream"),
    ("help-web", "web-songs"),
    ("help-pptx", "pptx-import"),
    ("help-songs", "new-song"),
    ("help-bible", "browse-bible"),
]


def keys_in(scene: str) -> set[str]:
    tree = Path(tempfile.gettempdir()) / f"artosia-check-help-{scene}.json"
    subprocess.run([str(EXE), "--scene", scene, "--tree", str(tree), "--size", "1366x768", "--scale", "1"], check=True, capture_output=True)
    found = set()

    def walk(node):
        if isinstance(node, dict):
            if isinstance(node.get("key"), str):
                found.add(node["key"])
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(json.loads(tree.read_text(encoding="utf-8")))
    return found


def main() -> None:
    missing = [(key, scene) for key, scene in PLACES if key not in keys_in(scene)]
    for key, scene in missing:
        print(f"FAIL {key} is not drawn in scene {scene}")
    print(f"{len(PLACES) - len(missing)} of {len(PLACES)} features show their help")
    sys.exit(1 if missing else 0)


if __name__ == "__main__":
    main()
