# keyfluent — project context

**Keyfluent**: native Rust (egui/eframe) desktop app for learning keyboard shortcuts by doing
(intent → keys, graded on speed, FSRS scheduling). tmux pack first, Vim later. Open source (MIT),
public repo `samuelkordik/keyfluent`. Read `PLAN.md`, then the design in
`docs/superpowers/specs/` and the phase plans in `docs/superpowers/plans/`.

## Rules
- **No network, ever.** No network crates (`cargo deny` enforces it), no telemetry. Disk writes
  only to the app-data directory (prefs, progress, review log, imported bindings).
- **Cards are tested content:** every card's canonical keys must reach its goal from its start
  state (core test). Never ship a card that fails it.
- Cards are tied to commands and goals, not keys (custom-binding import depends on it).
- Core crates have no UI code; the egui app only paints and forwards input.
- Use superpowers skills (brainstorming, writing-plans, TDD, verification) and the `ui-ux:*`
  skills for UI work (`DESIGN.md` + tokens drive every screen).

## Toolchain
Mirrors `~/dev/pco_reader`. All builds/tests run in the `keyfluent-dev` container:
`bin/dev <cmd>` (e.g. `bin/dev cargo test`). trantor is headless: UI is checked with
`egui_kittest` snapshot tests here and by Samuel on his Mac (`scripts/build-macos.sh`,
`scripts/mac/verify.sh`, `MAC_AGENT.md`). Unsigned macOS builds for now.

## Homelab
Registered in `~/dev/homelab/REGISTRY.md` (`keyfluent`: no ports, no hostnames) and on the
dashboard (project `keyfluent`, tasks `kf-*`). Close the phase task when a phase is done.
