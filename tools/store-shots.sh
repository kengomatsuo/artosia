#!/usr/bin/env bash
# Renders the Microsoft Store listing's screenshots from the real program, in every language.
#   tools/store-shots.sh                needs target/debug/artosia.exe
#   STORE_LANGS="ko ja" tools/store-shots.sh renders only those languages.
# Each picture is a headless render of a scene at 1366 by 768 points (2732 by 1536 pixels: the
# Store asks for at least 1366 by 768). The order is the order of the captions in
# tools/store/listing/<code>.json: window, search, design, screens, notices, first run.
set -euo pipefail
cd "$(dirname "$0")/.."

exe=target/debug/artosia.exe
out=target/store-shots
langs="${STORE_LANGS:-en id zh-Hans zh-Hant ko ja es pt-BR fr vi}"
mkdir -p "$out"

# One press of Next, so the first picture shows a song live on the projector.
cat > "$out/steps.json" <<'EOF'
{"events": [
  {"t": 0.50, "type": "key", "name": "Down"},
  {"t": 1.20, "type": "mark", "name": "live"}
]}
EOF

for lang in $langs; do
  mkdir -p "$out/$lang"
  shot() { "$exe" --scene "$1" --snap "$out/$lang/$2.png" --size 1366x768 --light --language "$lang" "${@:3}" > /dev/null 2>&1 || echo "failed: $lang $1"; }
  shot song 1-window --script "$out/steps.json" --at mark:live
  shot search-lyrics 2-search
  shot look 3-design
  shot screens-two 4-screens
  shot announcements 5-notices
  shot onboard-language 6-first-run
  echo "$lang: $(ls "$out/$lang" | wc -l) pictures"
done
