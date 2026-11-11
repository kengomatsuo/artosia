"""Drives Artosia for a tester who sees only screenshots.

A session is a list of things a person did: keys, clicks, typing. Every command replays the
whole list in a headless Artosia on the practice library and saves a picture of the window
as it then looks, so the tester decides the next step from the picture alone. Nothing here
touches a real window, the owner's library or the projector.

    python tools/blind.py new   <session> [--indonesian] [--light] [--size 1366x768] [--scene <name>]
    python tools/blind.py look  <session>                 the window as it is now
    python tools/blind.py key   <session> <Name> [ctrl] [shift] [alt]
    python tools/blind.py type  <session> "<text>"
    python tools/blind.py click <session> <x> <y>
    python tools/blind.py double <session> <x> <y>
    python tools/blind.py right <session> <x> <y>
    python tools/blind.py drag  <session> <x> <y> <dx> <dy>
    python tools/blind.py wheel <session> <x> <y> <dy>     dy below zero scrolls down
    python tools/blind.py move  <session> <x> <y>          rests the pointer there
    python tools/blind.py undo  <session>                  forgets the last step
    python tools/blind.py steps <session>                  lists what was done

Key names are those on the keyboard: Enter, Tab, Esc, Up, Down, Left, Right, Home, End,
PageUp, PageDown, Delete, Backspace, Space, F1 to F12, or one character. Coordinates are
pixels of the picture, from its top left corner. Each command prints the picture's path.
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# A step every 0.6 s of the app's own clock: long enough for a panel to finish opening.
STEP = 0.6
NAMES = {"esc": "Escape", "escape": "Escape", "space": " ", "enter": "Enter", "return": "Enter", "tab": "Tab", "up": "Up", "down": "Down", "left": "Left", "right": "Right", "home": "Home", "end": "End", "pageup": "PageUp", "pagedown": "PageDown", "delete": "Delete", "backspace": "Backspace"}


def folder(session):
    return os.path.join(os.environ.get("ARTOSIA_BLIND", os.path.join(ROOT, "target", "blind")), session)


def load(session):
    with open(os.path.join(folder(session), "session.json"), encoding="utf-8") as f:
        return json.load(f)


def save(session, state):
    os.makedirs(folder(session), exist_ok=True)
    with open(os.path.join(folder(session), "session.json"), "w", encoding="utf-8") as f:
        json.dump(state, f, indent=1)


def event(step, t):
    kind = step["do"]
    if kind == "key":
        out = {"t": t, "type": "key", "name": step["name"]}
        for mod in ("ctrl", "shift", "alt"):
            if step.get(mod):
                # Rune reads Ctrl on Windows as its command modifier.
                out["cmd" if mod == "ctrl" else mod] = True
                if mod == "ctrl":
                    out["ctrl"] = True
        return [out]
    if kind == "type":
        return [{"t": t, "type": "type", "text": step["text"]}]
    at = {"x": step["x"], "y": step["y"]}
    if kind == "click":
        return [{"t": t, "type": "tap", **at}]
    if kind == "double":
        return [{"t": t, "type": "double-click", **at}]
    if kind == "right":
        return [{"t": t, "type": "tap", "button": "secondary", **at}]
    if kind == "drag":
        return [{"t": t, "type": "drag", **at, "dx": step["dx"], "dy": step["dy"], "dur": 0.35}]
    if kind == "wheel":
        return [{"t": t, "type": "wheel", **at, "dx": 0, "dy": step["dy"]}]
    if kind == "move":
        return [{"t": t, "type": "move", **at}]
    raise SystemExit(f"unknown step {kind}")


def render(session, state):
    here = folder(session)
    events, t = [], 0.3
    for step in state["steps"]:
        events += event(step, round(t, 2))
        t += STEP
    events.append({"t": round(t, 2), "type": "mark", "name": "end"})
    script = os.path.join(here, "script.json")
    with open(script, "w", encoding="utf-8") as f:
        json.dump({"events": events}, f)
    shot = os.path.join(here, f"{len(state['steps']):03d}.png")
    # A frozen copy, when there is one, so a rebuild in the middle of a test changes nothing.
    name = "artosia.exe" if os.name == "nt" else "artosia"
    frozen = os.path.join(ROOT, "target", "blind", "bin", name)
    exe = os.environ.get("ARTOSIA_EXE") or (frozen if os.path.exists(frozen) else os.path.join(ROOT, "target", "debug", name))
    args = [exe, "--practice", "--script", script, "--snap", shot, "--at", "mark:end", "--size", state["size"], "--scale", "1"] + state["flags"]
    done = subprocess.run(args, capture_output=True, text=True, timeout=120)
    if not os.path.exists(shot):
        raise SystemExit("Artosia did not draw the window:\n" + done.stderr[-800:])
    print(shot)


def main():
    if len(sys.argv) < 3:
        raise SystemExit(__doc__)
    verb, session, rest = sys.argv[1], sys.argv[2], sys.argv[3:]
    if verb == "new":
        size = rest[rest.index("--size") + 1] if "--size" in rest else "1366x768"
        flags = [flag for flag in ("--indonesian", "--light", "--dark") if flag in rest]
        # A scene to start from, such as the first stop of the tour a new volunteer meets.
        if "--scene" in rest:
            flags += ["--scene", rest[rest.index("--scene") + 1]]
        state = {"size": size, "flags": flags, "steps": []}
        save(session, state)
        return render(session, state)
    state = load(session)
    if verb == "steps":
        for at, step in enumerate(state["steps"], 1):
            print(at, json.dumps(step))
        return
    if verb == "undo":
        if state["steps"]:
            state["steps"].pop()
    elif verb == "key":
        name = NAMES.get(rest[0].lower(), rest[0])
        step = {"do": "key", "name": name}
        for mod in rest[1:]:
            step[mod.lower()] = True
        state["steps"].append(step)
    elif verb == "type":
        state["steps"].append({"do": "type", "text": rest[0]})
    elif verb in ("click", "double", "right", "move"):
        state["steps"].append({"do": verb, "x": float(rest[0]), "y": float(rest[1])})
    elif verb == "drag":
        state["steps"].append({"do": "drag", "x": float(rest[0]), "y": float(rest[1]), "dx": float(rest[2]), "dy": float(rest[3])})
    elif verb == "wheel":
        state["steps"].append({"do": "wheel", "x": float(rest[0]), "y": float(rest[1]), "dy": float(rest[2])})
    elif verb != "look":
        raise SystemExit(__doc__)
    save(session, state)
    render(session, state)


if __name__ == "__main__":
    main()
