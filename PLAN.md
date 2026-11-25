# Keyfluent — plan

_2026-11-25. Design: [`docs/superpowers/specs/2026-11-25-keyfluent-design.md`](docs/superpowers/specs/2026-11-25-keyfluent-design.md)
(approved 2026-11-25; execution: subagent-driven). Each phase gets a detailed plan in `docs/superpowers/plans/`,
written when the phase starts so it targets the code that exists by then. Dashboard tasks: `kf-*`._

| Phase | Delivers | Plan | Dashboard |
|---|---|---|---|
| P0 Foundation | Workspace, dev container, lints, cargo-deny, CI, Mac build scripts, a **key-probe window** that shows raw key events (first Mac check: does every key arrive?) | [p0-foundation](docs/superpowers/plans/2026-11-25-p0-foundation.md) | `kf-p0` |
| UX plan | `ui-ux:ux-plan`: brief, 2–3 directions, **Samuel picks**, `DESIGN.md` + `tokens.json` | (skill-driven) | `kf-ux` |
| P1 Core + tmux simulator | Key notation, `list-keys` parser, stock tmux 3.4 tables, card format, tmux model (panes, windows, prompts, command prompt), goals, fixtures, **Basics** cards, canonical-keys content test | [p1-core-tmux](docs/superpowers/plans/2026-11-25-p1-core-tmux.md) | `kf-p1` |
| P2 Learning engine | Grading (steps, budget, speed target, ratings), FSRS via `rs-fsrs`, session builder (interleave, intro flow), storage (JSON, atomic, schema 1) | written at P2 start | `kf-p2` |
| P3 App | egui: Today, Practice (cell grid, goal picture, keystroke strip, result flash), intro flow, pause menu, input normalisation; **first usable build on the Mac** | written at P3 start | `kf-p3` |
| P4 Curriculum | Simulator: copy mode (vi + emacs), sessions, detach/fake shell, choose-tree, display-panes, layouts; cards for Copy mode, Sessions, Layout | written at P4 start | `kf-p4` |
| P5 Progress + prefs | Progress page (heatmap, history, calendar, forecast), Preferences, custom-binding import, wild mode, OS-shortcut flags, first-launch intro | written at P5 start | `kf-p5` |
| P6 Scenarios | Multi-step scenario cards + unlock rule | written at P6 start | `kf-p6` |
| P7 v0.1 | `ux-audit` + fixes, icon (`brand-identity`), macOS/Windows/Linux packages, README screenshots | written at P7 start | `kf-p7` |
| Later | Vim pack; `M-` cards; FSRS parameter optimisation; `keyfluent drill` in-terminal add-on (maybe) | — | `kf-vim` |
