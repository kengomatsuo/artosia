"""Records the guide's clips from the real program, one per lesson step.

    python -I tools/record-guide.py [step ...]

For each step: the operator renders headless at 1366x768 from the step's starting scene, a script
moves the pointer to the step's control and presses it, the frames go to target/guide-rec/<step>,
and examples/guide_clip.rs draws the pointer, crops to where the change happens and writes
assets/guide/<step>.png, an animated PNG the guide's bubble plays. Run it again whenever a screen
a lesson points at changes. Needs a debug build (`cargo build -p artosia --examples`).
"""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# ARTOSIA_EXE and ARTOSIA_CLIP name another build, such as one made from HEAD while the shared
# tree holds another session's unfinished work.
EXE = Path(os.environ.get("ARTOSIA_EXE", ROOT / "target" / "debug" / "artosia.exe"))
# Built once by `cargo build -p artosia --example guide_clip`; run as it is, so a tree another
# session is editing never has to compile in the middle of a recording.
CLIP = Path(os.environ.get("ARTOSIA_CLIP", ROOT / "target" / "debug" / "examples" / "guide_clip.exe"))
W, H = 1366, 768
# Whole window: the step changes the whole window (a tab); part: the change happens near the control.
STEPS = [
    ("screens-open", "start", "anchor-Screens-Bar-0", "part"),
    ("calendar-tab", "start", "tab-calendar", "whole"),
    ("calendar-new", "tab-calendar", "cal-new", "part"),
    ("people-tab", "start", "tab-people", "whole"),
    ("people-add", "tab-people", "people-add", "part"),
    ("giving-tab", "start", "tab-giving", "whole"),
    ("giving-add", "tab-giving", "giving-add", "part"),
    ("templates-tab", "start", "tab-templates", "whole"),
    ("templates-use", "tab-templates", "tpl-use", "whole"),
    ("notices-tab", "start", "tab-notices", "whole"),
    ("notices-new", "tab-notices", "notice-add", "part"),
    ("look-tab", "start", "tab-look", "whole"),
    ("plan-tab", "tab-calendar", "tab-plan", "whole"),
    ("plan-add", "start", "plan-add-button", "part"),
    ("phone-open", "start", "anchor-Remote-Bar-0", "part"),
    ("listen-open", "start", "anchor-Listen-Bar-0", "part"),
    ("stream-open", "start", "anchor-Stream-Bar-0", "part"),
    ("web-open", "start", "anchor-WebSongs-Panel-0", "whole"),
    # The PowerPoint button sits at the foot of the announcement list: the list is scrolled first.
    ("pptx-open", "tab-notices", "anchor-ImportPptx-Panel-0", "whole", "notice-list"),
]
START, STEP, PRESS = 0.2, 0.1, 1.4


def scrolled(over: str | None) -> list[dict]:
    """The events that scroll the list named `over` to its foot before the step starts."""
    if not over:
        return []
    return [{"t": 0.05, "type": "move", "key": over}, {"t": 0.1, "type": "wheel", "dx": 0, "dy": -4000}]


def rect_of(scene: str, key: str, over: str | None = None) -> list[float]:
    tree = ROOT / "target" / "guide-rec" / f"{scene}-tree.json"
    args = [str(EXE), "--scene", scene, "--indonesian", "--tree", str(tree), "--size", f"{W}x{H}", "--scale", "1"]
    if over:
        pre = ROOT / "target" / "guide-rec" / f"{scene}-pre.json"
        pre.write_text(json.dumps({"events": scrolled(over) + [{"t": 0.6, "type": "mark", "name": "end"}]}), encoding="utf-8")
        args += ["--script", str(pre), "--at", "0.6"]
    subprocess.run(args, check=True, capture_output=True)
    found = []

    def walk(node):
        if isinstance(node, dict):
            if node.get("key") == key and isinstance(node.get("rect"), list):
                found.append(node["rect"])
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(json.loads(tree.read_text(encoding="utf-8")))
    if not found:
        raise SystemExit(f"{key} is not drawn in scene {scene}")
    return found[0]


def crop_for(rect: list[float], kind: str) -> tuple[int, int, int, int]:
    if kind == "whole":
        return 0, 0, W, H
    cw, ch = 866, 487
    cx = rect[0] + rect[2] / 2
    x = int(min(max(cx - cw * 0.7, 0), W - cw))
    y = int(min(max(rect[1] - 40, 0), H - ch))
    return x, y, cw, ch


def record(step: str, scene: str, key: str, kind: str, over: str | None = None) -> None:
    rect = rect_of(scene, key, over)
    tx, ty = rect[0] + rect[2] / 2, rect[1] + rect[3] / 2
    sx, sy = W * 0.5, H * 0.62
    script = ROOT / "target" / "guide-rec" / f"{step}.json"
    script.write_text(json.dumps({"events": scrolled(over) + [
        {"t": 0.3, "type": "move", "x": sx, "y": sy},
        {"t": 0.9, "type": "move", "key": key},
        {"t": PRESS, "type": "tap", "key": key},
        {"t": 2.6, "type": "mark", "name": "end"},
    ]}), encoding="utf-8")
    frames = ROOT / "target" / "guide-rec" / step
    shutil.rmtree(frames, ignore_errors=True)
    frames.mkdir(parents=True)
    subprocess.run([str(EXE), "--scene", scene, "--indonesian", "--script", str(script), "--frames", "150", "--start", str(START), "--every", "6",
                    "--out", str(frames), "--size", f"{W}x{H}", "--scale", "1"], check=True, capture_output=True)
    x, y, cw, ch = crop_for(rect, kind)
    out = ROOT / "assets" / "guide" / f"{step}.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    done = subprocess.run([str(CLIP), "--frames", str(frames), "--out", str(out),
                           "--crop", f"{x},{y},{cw},{ch}", "--width", "400", "--start", str(START), "--step", str(STEP),
                           "--path", f"0.3:{sx},{sy};0.9:{tx},{ty}", "--press", str(PRESS),
                           "--pointer", "36" if kind == "part" else "48"], cwd=ROOT, check=True, capture_output=True, text=True)
    print(step, done.stdout.strip().splitlines()[-1])


def main() -> None:
    wanted = set(sys.argv[1:])
    for step, scene, key, kind, *over in STEPS:
        if not wanted or step in wanted:
            record(step, scene, key, kind, *over)


if __name__ == "__main__":
    main()
