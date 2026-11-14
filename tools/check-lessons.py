"""Walks every guide lesson in the real program and fails if one cannot reach its end.

    python -I tools/check-lessons.py [lesson ...]

For each lesson the operator renders headless from the scene `lesson-from-env`. The checker reads
the element tree, presses what the guide asks for (the ringed control, or Next on an explanation)
and renders again with every press so far, until the lesson's card is gone. A step whose card
stands in the middle with nothing ringed means the guide found no control for it: the lesson
fails there. Needs a debug build; ARTOSIA_EXE names another one, such as a build made from HEAD.
"""
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = Path(os.environ.get("ARTOSIA_EXE", ROOT / "target" / "debug" / "artosia.exe"))
LESSONS = ["basics", "screens", "calendar", "people", "giving", "templates", "notices", "look", "plan", "phone", "listen", "stream", "web", "pptx"]
# Long enough for the poll, which wakes once a second, to scroll a ringed control into view.
GAP = 1.6
WORK = Path(tempfile.gettempdir()) / "artosia-check-lessons"


def elements(tree) -> list[dict]:
    found = []

    def walk(node):
        if isinstance(node, dict):
            if "key" in node:
                found.append(node)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(tree)
    return found


def look(lesson: str, events: list[dict], at: float) -> list[dict]:
    script = WORK / f"{lesson}.json"
    tree = WORK / f"{lesson}-tree.json"
    script.write_text(json.dumps({"events": events + [{"t": at, "type": "mark", "name": "end"}]}), encoding="utf-8")
    env = dict(os.environ, ARTOSIA_LESSON=lesson)
    subprocess.run([str(EXE), "--scene", "lesson-from-env", "--script", str(script), "--tree", str(tree), "--at", str(at), "--size", "1366x768", "--scale", "1"],
                   check=True, capture_output=True, env=env)
    return elements(json.loads(tree.read_text(encoding="utf-8")))


def walk_lesson(lesson: str) -> tuple[bool, list[str]]:
    events: list[dict] = []
    seen: list[str] = []
    at = 0.8
    for _ in range(12):
        found = look(lesson, events, at)
        keys = {element.get("key"): element for element in found}
        step = next((key[len("lesson-step-"):] for key in keys if isinstance(key, str) and key.startswith("lesson-step-")), None)
        if step is None:
            return bool(seen), seen
        seen.append(step)
        # A person moves the pointer to a control before pressing it.
        if "lesson-next" in keys:
            events += [{"t": at + 0.1, "type": "move", "key": "lesson-next"}, {"t": at + 0.3, "type": "tap", "key": "lesson-next"}]
        elif "lesson-ring" in keys:
            x, y, w, h = keys["lesson-ring"]["rect"]
            events += [{"t": at + 0.1, "type": "move", "x": x + w / 2, "y": y + h / 2}, {"t": at + 0.3, "type": "tap", "x": x + w / 2, "y": y + h / 2}]
        elif "lesson-search" in keys:
            # The control is not on screen: the card opens the search with its name typed.
            events += [{"t": at + 0.1, "type": "move", "key": "lesson-search"}, {"t": at + 0.3, "type": "tap", "key": "lesson-search"},
                       {"t": at + 0.9, "type": "key", "name": "Enter"}]
            seen.append("by search")
        else:
            seen.append("nothing ringed")
            return False, seen
        at += GAP
        if len(seen) > 2 and seen[-1] == seen[-2] == seen[-3]:
            seen.append("stuck")
            return False, seen
    return False, seen + ["too long"]


def main() -> None:
    WORK.mkdir(parents=True, exist_ok=True)
    wanted = sys.argv[1:] or LESSONS
    failed = 0
    for lesson in wanted:
        passed, steps = walk_lesson(lesson)
        failed += not passed
        print(("pass " if passed else "FAIL ") + lesson + ": " + " > ".join(steps))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
