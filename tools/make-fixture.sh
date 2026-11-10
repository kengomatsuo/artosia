#!/bin/bash
# Builds fixtures/bg30.mp4: 15 s 1080p30 H.264, 8 Mb/s, faststart.
# Usage: FFMPEG=/path/to/ffmpeg tools/make-fixture.sh
set -euo pipefail
cd "$(dirname "$0")/.."
FF="${FFMPEG:-ffmpeg}"
"$FF" -v error -y -f lavfi -i "mandelbrot=size=1920x1080:rate=30,hue=H=t/2,format=yuv420p" -t 15 \
  -c:v libx264 -preset fast -b:v 8M -pix_fmt yuv420p -movflags +faststart fixtures/bg30.mp4
echo "wrote fixtures/bg30.mp4"
