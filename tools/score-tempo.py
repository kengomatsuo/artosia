"""Scores the tempo tracker on kept steps of real tracks.

    cargo build -p artosia-listen --example sources
    python tools/score-tempo.py [folder ...]

Each folder holds files named `<stated tempo>-<anything>.frames`, written by
`sources hear <seconds> keep <file>` while that track played (crates/artosia-listen/README.md,
"Measured on real music"). With no folder it reads every folder under fixtures/tempo-steps,
which git ignores. One line per track says whether the middle reading is the stated tempo,
half of it, double it or something else, then the counts.
"""
import glob
import os
import re
import subprocess
import sys

root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
exe = os.environ.get("SOURCES_EXE", os.path.join(root, "target", "debug", "examples", "sources.exe" if os.name == "nt" else "sources"))
folders = sys.argv[1:] or sorted(glob.glob(os.path.join(root, "fixtures", "tempo-steps", "*")))
files = sorted(found for folder in folders for found in glob.glob(os.path.join(folder, "*.frames")))
if not files:
    sys.exit("no kept steps found; see the README of crates/artosia-listen")
out = subprocess.run([exe, "read", *files], capture_output=True, text=True, check=True).stdout
kinds = (("right", 1.0), ("double", 2.0), ("half", 0.5), ("x1.5", 1.5), ("x0.67", 2 / 3), ("x0.75", 0.75), ("x1.33", 4 / 3))
tally = {}
for line in out.splitlines():
    named = re.match(r"(\d+)-(\S+)\.frames: tempo: (.*)", line)
    if not named:
        continue
    stated, name, rest = float(named.group(1)), named.group(2), named.group(3)
    read = re.match(r"first reading at ([\d.]+) s, middle ([\d.]+) BPM, (\d+) of (\d+)", rest)
    if not read:
        kind, shown = "unread", "no reading"
    else:
        first, middle, near, total = float(read.group(1)), float(read.group(2)), read.group(3), read.group(4)
        kind = next((label for label, ratio in kinds if abs(middle / stated - ratio) / ratio < 0.03), "other")
        shown = f"{middle:6.1f} first at {first:4.1f} s, {near} of {total} steady"
    tally[kind] = tally.get(kind, 0) + 1
    print(f"{int(stated):4d}  {name[:14]:14s} {kind:7s} {shown}")
print(", ".join(f"{count} {kind}" for kind, count in sorted(tally.items())), "of", sum(tally.values()))
