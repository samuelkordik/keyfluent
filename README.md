# Keyfluent

Learn keyboard shortcuts by doing. Keyfluent shows you a goal (for example, "split this pane
side by side"), and you press the keys that get you there in a realistic simulator. It times
you, grades you, and schedules reviews with spaced repetition (FSRS), so a few minutes a day
turns the keys into muscle memory.

- **First pack: tmux.** From zero: panes, windows, copy mode, sessions. Teaches the stock
  bindings, or your own imported from `tmux list-keys`.
- **Next: Vim.**
- Native desktop app (Rust + egui) for macOS, Windows and Linux, so every key reaches the
  app. No account, no network; your progress stays on your computer.

**Status:** foundation done (P0); no lessons yet. See [`STATUS.md`](STATUS.md), [`PLAN.md`](PLAN.md) and the
[design](docs/superpowers/specs/2026-11-25-keyfluent-design.md).

## Build

Development builds and tests run in a Docker container (the dev machine is headless):

```sh
bin/dev cargo test
bin/dev cargo clippy --all-targets -- -D warnings
```

`bin/dev cargo run` has no display, so to see the app run it on a desktop with Rust installed:

```sh
cargo run -p kf-app                    # the app
cargo run -p kf-app -- --key-probe     # key probe: shows which keys reach the app
```

On macOS, `scripts/build-macos.sh` builds the app. It is unsigned, so on first launch
right-click the app and choose Open. See [`MAC_AGENT.md`](MAC_AGENT.md) for the Mac checks.

## License

MIT. See [`LICENSE`](LICENSE).
