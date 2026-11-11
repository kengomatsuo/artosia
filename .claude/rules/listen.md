---
paths:
  - "crates/artosia-listen/**"
---

# Listening to the room (2026-11-11)

- **THE TEMPO TRACKER IS PLAIN RUST WITH NO MODEL, AND ITS TESTS USE MADE-UP SIGNALS ONLY (`crates/artosia-listen/src/tempo.rs`, 2026-11-11, owner-asked).** It reports nothing for speech and noise; its limits (half-time and double-time readings, music with soft attacks) are measured in the README. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
- **A SOURCE IS SAVED BY ITS ID AND FOUND BY ID OR NAME; "system-audio" IS WASAPI LOOPBACK OF THE DEFAULT OUTPUT AND DELIVERS NOTHING WHILE NOTHING PLAYS, SO THE LISTENER FEEDS SILENCE (`crates/artosia-listen/src/capture.rs`, `lib.rs`, 2026-11-11, owner-asked).** The tempo tracker is held to within 2 BPM on five made-up rhythms at ten tempos by `tests/tempo_accuracy.rs`. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
