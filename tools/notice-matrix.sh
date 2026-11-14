#!/usr/bin/env bash
# Draws every birthday card setting in every announcement style at projector size, from the
# states examples/notice_matrix.rs writes, and lays each setting out as one contact sheet.
#   tools/notice-matrix.sh [id|en]        -> fixtures/notice-matrix/<language>/sheets/*.png
set -euo pipefail
cd "$(dirname "$0")/.."
language="${1:-id}"
out="fixtures/notice-matrix/$language"
rm -rf "$out"
cargo build -q -p artosia-output
cargo run -q -p artosia-output --example notice_matrix -- "$out/json" "$language"
mkdir -p "$out/png"
ls "$out/json" | sed 's/\.json$//' | xargs -P 4 -I{} sh -c \
  "./target/debug/artosia-output --window audience --scene state-file --state-file '$out/json/{}.json' --snap '$out/png/{}.png' --size 1920x1080 --scale 0.5 >/dev/null 2>&1"
python -I tools/contact-sheet.py "$out/png" "$out/sheets"
