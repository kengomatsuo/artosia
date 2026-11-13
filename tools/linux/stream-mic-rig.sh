#!/usr/bin/env bash
# Run on Linux (the release PC's WSL): tools/linux/stream-mic-rig.sh [facts]
# A microphone to test with. WSL's own cannot be opened, so a sound server of
# our own (PipeWire with its PulseAudio service, in a scratch runtime folder so the session's
# own is untouched) gets an input named "mic", set as the default, that carries a tone. The
# streaming program must list it as the default input and record its sound with no --audio.
set -u
cd ~/artosia || exit 1
command -v pactl > /dev/null && command -v pipewire-pulse > /dev/null || sudo -n DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends pipewire-pulse pulseaudio-utils 2>&1 | grep -E "^E:|Need to get|After this" | cut -c1-160
out=~/artosia-check/mic; rm -rf "$out"; mkdir -p "$out/run"; chmod 700 "$out/run"
bin="${BIN:-$HOME/artosia/target/debug}"; echo "programs from: $bin"
started=()
finish() { for pid in "${started[@]}"; do kill "$pid" 2>/dev/null; done; sleep 1; for pid in "${started[@]}"; do kill -9 "$pid" 2>/dev/null; done; }
trap finish EXIT
Xvfb :92 -screen 0 1920x1080x24 -nolisten tcp > "$out/xvfb.log" 2>&1 & started+=($!)
sleep 2
export DISPLAY=:92; unset WAYLAND_DISPLAY
"$bin/artosia-output" --demo-video "$HOME/artosia/fixtures/bg30.mp4" --here --port 47939 > "$out/projector.log" 2>&1 & started+=($!)
export XDG_RUNTIME_DIR="$out/run"
pipewire > "$out/pipewire.log" 2>&1 & started+=($!)
sleep 1
wireplumber > "$out/wireplumber.log" 2>&1 & started+=($!)
pipewire-pulse > "$out/pipewire-pulse.log" 2>&1 & started+=($!)
sleep 3
export PULSE_SERVER="unix:$out/run/pulse/native"
pactl info 2>&1 | grep -E "Server Name" | cut -c1-90
pactl load-module module-null-sink sink_name=room > /dev/null 2>&1 || echo "no null sink"
pactl load-module module-remap-source master=room.monitor source_name=mic source_properties=device.description=Microphone > /dev/null 2>&1 || echo "no remap source"
pactl set-default-source mic 2>&1 | head -1
echo "sources: $(pactl list sources short 2>/dev/null | awk '{print $2}' | tr '\n' ' '); default: $(pactl get-default-source 2>/dev/null)"
gst-launch-1.0 -q audiotestsrc wave=sine freq=440 volume=0.3 is-live=true ! pulsesink device=room > "$out/tone.log" 2>&1 & started+=($!)
sleep 8
"$bin/artosia-stream" --list > "$out/list.json" 2> "$out/list.err"; echo "list exit: $?; bytes: $(stat -c %s "$out/list.json")"
head -3 "$out/list.err" | cut -c1-200
python3 - "$out/list.json" <<'PY'
import json, sys
text = open(sys.argv[1]).read().strip()
if text:
    d = json.loads(text.splitlines()[-1])
    print('listed inputs (name, default, plays-the-computer):', [(a['name'][:28], a['default'], a['loopback']) for a in d['audio_inputs']])
PY
python3 - <<'PY'
import gi
gi.require_version("Gst", "1.0")
from gi.repository import Gst
Gst.init(None)
monitor = Gst.DeviceMonitor.new()
monitor.add_filter("Audio/Source", None)
print("providers:", monitor.get_providers())
for device in monitor.get_devices():
    props = device.get_properties()
    keep = {}
    if props:
        for i in range(props.n_fields()):
            name = props.nth_field_name(i)
            if name in ("is-default", "device.class", "device.api", "media.class", "node.name"): keep[name] = props.get_value(name)
    element = device.create_element(None)
    names = [p.name for p in element.list_properties()]
    target = {n: element.get_property(n) for n in ("device", "path", "target-object") if n in names}
    print("device:", device.get_display_name(), "|", device.get_device_class(), "|", keep, "|", element.get_factory().get_name(), target)
PY
if [ "${1:-}" = facts ]; then exit 0; fi
rm -f "$out/stop" "$out/mic.mp4"
"$bin/artosia-stream" --window "Artosia projector" --record "$out/mic.mp4" --stop-file "$out/stop" > "$out/mic.log" 2> "$out/mic.err" &
pid=$!
sleep 12; touch "$out/stop"; waited=0
while kill -0 $pid 2>/dev/null && [ $waited -lt 20 ]; do sleep 1; waited=$((waited + 1)); done
kill -9 $pid 2>/dev/null
echo "said: $(grep -v '"type":"status"' "$out/mic.log" | grep -o '"message":"[^"]*"' | cut -c12- | tr '\n' ' ' | cut -c1-260); dropped: $(grep -o '"frames_dropped":[0-9]*' "$out/mic.log" | tail -1)"
echo "file: $(stat -c %s "$out/mic.mp4" 2>/dev/null) bytes; $(ffprobe -v error -show_entries stream=codec_name:format=duration -of csv=p=0 "$out/mic.mp4" 2>&1 | grep -v '^$' | tr '\n' ' '); sound: $(ffmpeg -v info -i "$out/mic.mp4" -vn -af volumedetect -f null - 2>&1 | grep -o "mean_volume: [-0-9.]* dB")"
