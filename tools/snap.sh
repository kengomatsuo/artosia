#!/usr/bin/env bash
# Renders operator scenes to PNG without opening a window, for checking a screen by eye.
#   tools/snap.sh <out-dir> [scene ...]      no scene names renders every scene
# Extra flags go through SNAP_FLAGS, for example SNAP_FLAGS="--size 1920x1080".
set -euo pipefail
out="${1:?usage: tools/snap.sh <out-dir> [scene ...]}"
shift
exe="$(dirname "$0")/../target/debug/artosia"
[ -x "$exe.exe" ] && exe="$exe.exe"
mkdir -p "$out"
if [ "$#" -eq 0 ]; then
  set -- $("$exe" --list-scenes | tr -s ' \r\n' ' ')
fi
for scene in "$@"; do
  # shellcheck disable=SC2086
  "$exe" --scene "$scene" --snap "$out/$scene.png" ${SNAP_FLAGS:-} >/dev/null 2>&1 || echo "failed: $scene"
done
echo "rendered $# scenes into $out"
