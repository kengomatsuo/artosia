#!/bin/bash
# Builds fixtures/bg30.mp4: 15 s 1080p30 H.264, 8 Mb/s, faststart.
# Usage: FFMPEG=/path/to/ffmpeg tools/make-fixture.sh
set -euo pipefail
cd "$(dirname "$0")/.."
FF="${FFMPEG:-ffmpeg}"
"$FF" -v error -y -f lavfi -i "mandelbrot=size=1920x1080:rate=30,hue=H=t/2,format=yuv420p" -t 15 \
  -c:v libx264 -preset fast -b:v 8M -pix_fmt yuv420p -movflags +faststart fixtures/bg30.mp4
echo "wrote fixtures/bg30.mp4"

# Rundown items: a 4 s 1080p30 clip with a 440 Hz tone, and a 5 s 220 Hz tone for music.
"$FF" -v error -y -f lavfi -i "mandelbrot=size=1920x1080:rate=30,hue=H=t,format=yuv420p" -f lavfi -i "sine=frequency=440:sample_rate=48000" -t 4 \
  -c:v libx264 -preset fast -b:v 8M -pix_fmt yuv420p -c:a aac -b:a 96k -shortest -movflags +faststart fixtures/clip-sound.mp4
"$FF" -v error -y -f lavfi -i "sine=frequency=220:sample_rate=48000" -t 5 fixtures/tone.wav
echo "wrote fixtures/clip-sound.mp4 and fixtures/tone.wav"
