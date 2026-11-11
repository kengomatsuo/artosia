#!/usr/bin/env bash
# Renders the homepage's pictures from the real programs, in both languages and themes.
#   tools/site-shots.sh        needs target/debug/artosia.exe and artosia-output.exe
# Every picture is a headless render of a scene; a dialog is cut out with a little
# of the dimmed window around it. Rerun after a screen changes, then look at site/img.
set -euo pipefail
cd "$(dirname "$0")/.."

exe=target/debug/artosia.exe
out=site/img
tmp=target/site-shots
mkdir -p "$tmp" "$out/en" "$out/id"

# Three presses of Next, with a mark before and after each.
cat > "$tmp/steps.json" <<'EOF'
{"events": [
  {"t": 0.50, "type": "mark", "name": "f0"},
  {"t": 0.80, "type": "key", "name": "Down"},
  {"t": 1.40, "type": "mark", "name": "f1"},
  {"t": 1.70, "type": "key", "name": "Down"},
  {"t": 2.30, "type": "mark", "name": "f2"},
  {"t": 2.60, "type": "key", "name": "Down"},
  {"t": 3.20, "type": "mark", "name": "f3"}
]}
EOF

# cut <scene> <name> <x0> <y0> <x1> <y1>: a dialog out of a 1280 by 800 window
cut() {
  "$exe" --scene "$1" --snap "$tmp/$1.png" --size 1280x800 "--$theme" $flag > /dev/null 2>&1
  powershell -NoProfile -File tools/crop-png.ps1 -In "$tmp/$1.png" -Out "$out/$lang/$2-$theme.png" \
    -X0 "$3" -Y0 "$4" -X1 "$5" -Y1 "$6"
}

for lang in en id; do
  flag=""; [ "$lang" = id ] && flag="--indonesian"
  for theme in light dark; do
    for n in 0 1 2 3; do
      "$exe" --scene song --script "$tmp/steps.json" --at "mark:f$n" \
        --snap "$out/$lang/hero-$n-$theme.png" --size 1280x800 "--$theme" $flag > /dev/null 2>&1
    done
    cut search-lyrics          search        0.1235 0.04 0.8765 0.82
    cut look-styled            design        0.215  0.05 0.785  0.95
    cut screens-two            screens       0.215  0.05 0.785  0.95
    cut announcement-birthdays announcements 0.2    0.06 0.78   0.955
  done
done

target/debug/artosia-output.exe --window audience --scene lyrics --snap "$out/projector.png" \
  --size 1920x1080 --scale 1 > /dev/null 2>&1

# The picture a shared link shows: the top of the dark window at 1200 by 630.
powershell -NoProfile -File tools/crop-png.ps1 -In "$out/en/hero-0-dark.png" -Out "$out/og.png" \
  -X0 0 -Y0 0 -X1 1 -Y1 0.84 -Width 1200

echo "pictures in $out"
