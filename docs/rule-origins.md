# Where the rules came from

The rule files in `.claude/rules/` and `CLAUDE.md` say what holds and why. The day a rule was made, who asked for it and what went wrong first are told in the page each rule links as its full account, and on the board (`docs/plans/artosia/BOARD.md`). This page keeps only what no other page tells.

## When each rule file began

| Rule file | Began |
|---|---|
| `core.md` (songs, Bibles and search) | 2026-11-10 |
| `operator.md` (the operator window) | 2026-11-10 |
| `output.md` (the output process and what the screens draw) | 2026-11-10 |
| `rune.md` (the pinned UI framework) | 2026-11-10 |
| `listen.md` (listening to the room) | 2026-11-11 |
| `remote.md` (the phone remote) | 2026-11-11 |
| `site.md` (the homepage) | 2026-11-11 |
| `setup.md` (the setup program) | 2026-11-12 |
| `store.md` (the Microsoft Store edition) | 2026-11-12 |
| `stream.md` (the streaming program) | 2026-11-13 |

## Incidents told nowhere else

- **Check a screen by rendering it.** The owner's call, 2026-11-10.
- **A RAM number counts only if the video was proven to loop.** Reported by the owner, 2026-10-06: a WPF test was measured after its video had silently stopped ([spikes/SPEC.md](../spikes/SPEC.md)).
- **The phone page copies Proclaim Remote.** The owner reported on 2026-11-11 that the first page looked ugly, and on 2026-11-12 picked Proclaim Remote from four candidates built from saved references.
- **A test of the remote listens on this computer only.** An example listened on the network, Windows Firewall asked whether to allow it, and two such questions sat on the owner's screen for hours taking every real key and click. The firewall log then held 13 such questions in two days (2026-11-12).
- **An announcement change is checked with the notice matrix.** 2026-11-15, card BDAYCHECK1, reported by the owner: the first run of the matrix found nine defects no scene showed.
- **A sound change is recorded on a PipeWire server with a default input of our own.** 2026-11-13: WSL's own sound server had no microphone that opened, so it showed nothing of what a real desktop does.
- **A question to X11 has two seconds to answer.** 2026-11-13: WSL's X display took a connection and never answered, which held the stream and its stopping.
- **Only this program's own sound inputs are listed.** 2026-11-13: release 0.1.25 streamed silence on a PipeWire desktop.
- **On Wayland a window is shown at once.** 2026-11-13: from 0.1.22 to 0.1.26 the projector never appeared under Wayland.
- **A new screen is rendered in Indonesian too.** 2026-11-10: three layouts clipped the first time.
- **The plan column has one width.** Reported by the owner, 2026-11-11: two constants, 256 and 272, put the drop-down's box 8 points outside the column.
- **One heard phrase names a song only under two conditions.** 2026-11-11: the first rule offered five songs in 100 seconds of a sermon.
- **A tempo press says what the tempo is.** 2026-11-11: a multiplier alone was dropped each time the tracker flipped between 60 and 120 within one ballad.
