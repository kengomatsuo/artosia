"""Reads the recordings a church handed over and says how the listener did.

    python tools/learn-from.py <recordings folder or .zip> [...]

The folder is what Artosia keeps when "Keep recordings" is on in Listen (docs/recordings.md):
one folder a sitting, each with about.json, attempts.jsonl and the phrases as wave files.
What was on the projector is the answer the guesses are measured against, so a sitting only
teaches something while a song was shown.

For each sitting it prints, song by song: how many phrases were read while it was shown, how
often the song was among a phrase's nearest three, how often it was named, how often another
song was named, and the tempo read; then which wave files to listen to, the phrases no words
were read in and the ones whose words led away from the song. Then the totals. It prints
titles, never the words heard.
"""
import json
import os
import statistics
import sys
import tempfile
import zipfile


def sittings(path):
    """Yields (name, about, events) for every sitting under a folder or inside a zip."""
    if path.lower().endswith(".zip"):
        folder = tempfile.mkdtemp(prefix="artosia-learn-")
        with zipfile.ZipFile(path) as packed:
            # Only the small text files are needed here; the sound stays in the zip.
            for name in packed.namelist():
                if name.endswith(("about.json", "attempts.jsonl")):
                    packed.extract(name, folder)
        path = folder
    for root, _, files in sorted(os.walk(path)):
        if "attempts.jsonl" not in files or os.sep + ".deleted" in root:
            continue
        about = {}
        if "about.json" in files:
            about = json.load(open(os.path.join(root, "about.json"), encoding="utf-8"))
        events = [json.loads(line) for line in open(os.path.join(root, "attempts.jsonl"), encoding="utf-8") if line.strip()]
        yield os.path.basename(root), about, events


def study(events):
    """Song by song, what the listener made of the phrases heard while that song was shown."""
    songs, order = {}, []
    live = None
    for event in events:
        kind = event.get("kind")
        if kind == "live":
            live = event.get("title") if event.get("what") == "song" else None
            if live and live not in songs:
                songs[live] = {"phrases": 0, "words": 0, "near": 0, "named": 0, "other": 0, "tempo": [], "counts": [], "silent": [], "astray": []}
                order.append(live)
        elif live and kind == "read":
            row = songs[live]
            row["phrases"] += 1
            close = live in [near.get("title") for near in event.get("near", [])]
            if event.get("heard"):
                row["words"] += 1
                if not close:
                    row["astray"].append(event.get("n"))
            else:
                row["silent"].append(event.get("n"))
            if close:
                row["near"] += 1
            named = event.get("named", [])
            if live in named:
                row["named"] += 1
            elif named:
                row["other"] += 1
        elif live and kind == "tempo" and event.get("bpm"):
            songs[live]["tempo"].append(event["bpm"])
        elif live and kind == "count":
            songs[live]["counts"].append(event.get("to"))
    return [(title, songs[title]) for title in order]


total = {"phrases": 0, "words": 0, "near": 0, "named": 0, "other": 0}
seen = 0
for path in sys.argv[1:]:
    for name, about, events in sittings(path):
        seen += 1
        kept = [event for event in events if event.get("kind") == "phrase"]
        seconds = sum(event.get("seconds", 0) for event in kept)
        taken = [event for event in events if event.get("kind") == "taken"]
        print(f"\n{name}: Artosia {about.get('artosia', '?')}, source {about.get('source', '?')}, language {about.get('language', '?')}, words {'read' if about.get('words') else 'not read'}, {about.get('songs', '?')} songs in the library")
        print(f"  {len(kept)} phrases kept, {seconds / 60:.1f} minutes of sound; {len(taken)} offers taken")
        rows = study(events)
        if not rows:
            print("  no song was shown, so there is nothing to measure the guesses against")
        for title, row in rows:
            tempo = f"{statistics.median(row['tempo']):.0f} BPM" if row["tempo"] else "no tempo"
            pressed = f", counted {'/'.join(row['counts'])}" if row["counts"] else ""
            print(f"  {title[:44]:44s} {row['phrases']:3d} phrases, {row['words']:3d} with words, near {row['near']:3d}, named {row['named']:3d}, another named {row['other']:3d}; {tempo}{pressed}")
            for label, numbers in (("no words read in", row["silent"]), ("words led away in", row["astray"])):
                if numbers:
                    print(f"    {label} phrase {', '.join(str(n) for n in numbers)}")
            for key in total:
                total[key] += row[key]
if not seen:
    sys.exit("no sitting found: give the recordings folder, or the zip Artosia made of it")
if total["phrases"]:
    of_all = f"{total['words']} ({100 * total['words'] // total['phrases']}%)"
    share = lambda part: f"{part} ({100 * part // max(total['words'], 1)}%)"
    print(f"\nWhile a song was shown: {total['phrases']} phrases, {of_all} with words. Of those with words: the song among the nearest three {share(total['near'])}; the song named {share(total['named'])}; another song named {share(total['other'])}")
