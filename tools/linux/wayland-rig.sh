#!/usr/bin/env bash
# Run on Linux (the release PC's WSL): tools/linux/wayland-rig.sh
# The projector's window on a Wayland desktop (sway with no screen; needs sway, Xvfb, xdotool).
#  1. Wayland alone: the window must appear and draw.
#  2. An X11 server that answers beside it (Xvfb): the window must go there.
#  3. The same with ARTOSIA_WAYLAND=1: it must stay on Wayland.
#  4. An X11 display that never answers beside it (WSL's own): Wayland, after two seconds.
set -u
cd ~/artosia || exit 1
out=~/artosia-check/projector-wayland; rm -rf "$out"; mkdir -p "$out"
export XDG_RUNTIME_DIR=/run/user/$(id -u)
started=()
finish() { for pid in "${started[@]}"; do kill "$pid" 2>/dev/null; done; sleep 1; for pid in "${started[@]}"; do kill -9 "$pid" 2>/dev/null; done; }
trap finish EXIT
printf 'output HEADLESS-1 resolution 1920x1080\ndefault_border none\n' > "$out/sway.conf"
before=$(ls "$XDG_RUNTIME_DIR" | grep -E "^wayland-[0-9]+$" | tr '\n' ' ')
env -u WAYLAND_DISPLAY -u DISPLAY WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman sway -c "$out/sway.conf" > "$out/sway.log" 2>&1 &
started+=($!)
Xvfb :91 -screen 0 1920x1080x24 -nolisten tcp > "$out/xvfb.log" 2>&1 & started+=($!)
sleep 3
socket=""
for name in $(ls "$XDG_RUNTIME_DIR" | grep -E "^wayland-[0-9]+$"); do case " $before " in *" $name "*) ;; *) socket="$name" ;; esac; done
[ -n "$socket" ] || { echo "sway made no socket"; exit 1; }
ipc=$(ls -t $XDG_RUNTIME_DIR/sway-ipc.*.sock | head -1)
in_sway() { swaymsg -s "$ipc" -t get_tree 2>/dev/null | python3 -c "
import json, sys
def walk(n, found):
    if n.get('pid'): found.append((n.get('name'), n.get('shell'), n.get('rect', {}).get('width'), n.get('rect', {}).get('height')))
    for c in n.get('nodes', []) + n.get('floating_nodes', []): walk(c, found)
found = []; walk(json.load(sys.stdin), found); print(found)
"; }
try() { # try <name> <environment...>
    local name="$1"; shift
    env -u DISPLAY -u WAYLAND_DISPLAY "$@" ARTOSIA_TRACE=1 target/debug/artosia-output --demo-video "$HOME/artosia/fixtures/bg30.mp4" --here --port 47939 > "$out/$name.log" 2>&1 &
    local pid=$!; started+=($pid)
    sleep 14
    echo "$name: on sway $(in_sway); on the X11 screen $(DISPLAY=:91 xdotool search --name "Artosia projector" 2>/dev/null | wc -l) window; $(grep "frames projector" "$out/$name.log" | tail -1 | grep -o "totals [0-9]* presents"); $(grep -E "windows: through|places windows itself|window shown|covers" "$out/$name.log" | head -2 | sed 's/^\[[^]]*\] //' | tr '\n' '|' | cut -c1-200)"
    kill $pid 2>/dev/null; sleep 2; kill -9 $pid 2>/dev/null
}
try wayland-alone WAYLAND_DISPLAY="$socket"
try x11-answers WAYLAND_DISPLAY="$socket" DISPLAY=:91
try told-to-stay WAYLAND_DISPLAY="$socket" DISPLAY=:91 ARTOSIA_WAYLAND=1
try x11-silent WAYLAND_DISPLAY="$socket" DISPLAY=:0
