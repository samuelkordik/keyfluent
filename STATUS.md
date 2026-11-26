# Status (2026-11-26)

**P0 (foundation) is done:** dev container (`bin/dev`), workspace (`kf-core`, `kf-tmux`,
`kf-app`), key-probe window, `cargo deny` + CI, macOS build and verify scripts.

2026-11-25: **P1 (core + tmux simulator) is done** on branch `p1-core-tmux`, with 17 Basics cards.

2026-11-26: **P2 (learning engine) is done** on branch `p2-learning-engine`: grading (stars, Ruling 16
cap), personal reaction time, FSRS scheduling and mastery bands, prefs and storage (`prefs.json`,
`progress.json`, `reviews.jsonl`), session building and running, combo, session summary and
calendar stats, all in `kf-core`. `crates/kf-tmux/tests/session.rs` runs whole sessions over the
Basics pack end to end and is the reference wiring for the app.

**Next:** the egui practice screens in P3 (see the P3 items in `docs/superpowers/notes/p1-followups.md`).
