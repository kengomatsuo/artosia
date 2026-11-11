---
paths:
  - "crates/artosia-listen/**"
---

# Listening to the room (2026-11-11)

- **THE TEMPO TRACKER IS PLAIN RUST WITH NO MODEL, AND ITS TESTS USE MADE-UP SIGNALS ONLY (`crates/artosia-listen/src/tempo.rs`, 2026-11-11, owner-asked).** It reports nothing for speech and noise; its limits (half-time and double-time readings, music with soft attacks) are measured in the README. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
- **A SOURCE IS SAVED BY ITS ID AND FOUND BY ID OR NAME; "system-audio" IS WASAPI LOOPBACK OF THE DEFAULT OUTPUT AND DELIVERS NOTHING WHILE NOTHING PLAYS, SO THE LISTENER FEEDS SILENCE (`crates/artosia-listen/src/capture.rs`, `lib.rs`, 2026-11-11, owner-asked).** The tempo tracker is held to within 2 BPM on five made-up rhythms at seven tempos up to 128 BPM by `tests/tempo_accuracy.rs`. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
- **A PROBLEM LEAVES THIS CRATE AS A KIND (`Problem`, `ListenError`), NEVER A SENTENCE; THE OPERATOR WORDS IT IN `crates/artosia/src/why.rs` (2026-11-11).** The Listen dialog showed "No speech model installed (looked for C:\...)" in English under an Indonesian heading. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
- **A CHANGE TO THE TEMPO TRACKER IS SCORED ON KEPT STEPS OF REAL MUSIC AS WELL AS ON THE MADE-UP TABLE (`tools/score-tempo.py` over `fixtures/tempo-steps`, written by `examples/sources.rs` `hear <seconds> keep <file>`, 2026-11-11, owner-asked).** The steps are rises per band, never sound, and stay out of the repository. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
- **THE TRACKER REPORTS NO TEMPO ABOVE 132 BPM (`tempo.rs` `MAX_BPM`, 2026-11-11, measured on 35 real tracks).** A slow song's eighth notes read as its beat (68 as 136) until its double was taken off the table; a faster song now reads at half speed. Full account: [crates/artosia-listen/README.md](../../crates/artosia-listen/README.md)
