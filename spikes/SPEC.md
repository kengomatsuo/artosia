# Output-video spike spec

Both spikes build the same output window so their RAM and smoothness can be compared.

## The window
- Borderless 1920x1080 at (0,0), black.
- `fixtures/bg30.mp4` (build it with `tools/make-fixture.sh`: 15 s, H.264 1080p30, faststart) fills it (cover), muted, looping forever.
- Lyrics drawn over the video: white, semibold, 64 px, centred, ~8% side margins. Every 3 s fade out over 400 ms, switch line, fade in. Lines:
  "Amazing grace, how sweet the sound", "That saved a wretch like me", "I once was lost, but now am found",
  "Was blind, but now I see", "Kasih karunia-Mu ajaib", "주 하나님 지으신 모든 세계"
- Every 2 s append `pos <playback seconds>` to `media.log` next to the spike.

## Proof before measuring
- Run 36 s; `media.log` must climb to ~14 and wrap back near 0 at least twice. A WPF test in the stack survey stopped after one pass because its file had no duration header, and the RAM number taken after it stopped was wrong.
- Look at it (screenshot or Rune `--snap`): video and text both visible.

## Measure
`powershell -File tools/ram/measure.ps1 -Exe <exe> -Dir <spike dir> -Wait 40 -Label <name>`, twice per spike, one app at a time, nothing else running.
