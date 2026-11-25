# Keyfluent — design

_2026-11-25. Status: **approved by Samuel 2026-11-25.**_

Keyfluent is a free, open-source desktop app (Rust + egui; macOS, Windows, Linux) for learning
keyboard shortcuts by doing. The app shows a **goal**, you press the keys that reach it, and
it grades you on correctness and speed. FSRS spaced repetition schedules the reviews. The
first pack is **tmux**; **Vim** comes later as a second pack.

## 1. Intent

- **Why it exists.** Samuel is a complete tmux beginner. He uses tmux on trantor over SSH
  from his Mac. The basics and copy mode trip him up. He is getting comfortable with sessions
  and switching, but still looks them up. He wants muscle memory: keys that come without
  thinking, from a few minutes of practice once or twice a day.
- **Who it's for.** Samuel first, but **complete beginners in general**. Someone who has never
  pressed `Ctrl-b` must be able to open the app and start learning without reading docs.
- **The core idea: intent → keys.** Real use starts with "I want this pane split". It doesn't
  start with "what does `%` do". Every card shows what to achieve, never the key name, and you
  answer by pressing keys in a realistic simulator.
- **Why a native app.** A browser can't capture every key: Chrome reserves `Ctrl-W`/`T`/`N`,
  and Option-key combinations on a Mac are hard to read. A native window receives nearly
  everything. A real GUI, rather than a terminal app, makes preferences and progress easy for
  beginners.

## 2. Goals, non-goals, anti-goals

**Goals**
1. Teach the stock tmux bindings, starting from zero, by intent → keys practice in a simulator.
2. Grade automatically from correctness and time. Never ask the learner to rate themselves.
3. Schedule with FSRS, with speed as the measure of fluency.
4. Combine learned bindings into multi-step **scenarios** once the single cards are solid.
5. Teach **custom** bindings by importing `tmux list-keys` output.
6. Show progress per binding on a **progress page**.
7. Be built from the start so more tools can be added as packs. Vim is next.
8. Be open source (MIT), private, and offline.

**Non-goals (v1)**
- Vim, Emacs or other packs. The architecture must allow them; the content comes later.
- Reminders or notifications. Off, and not built.
- A session timer. Practice is as long as you like.
- Sync, accounts, cloud, telemetry. Progress lives on one computer.
- Signed or notarized macOS builds. Unsigned until Samuel has an Apple Developer account.
- Teaching tmux *configuration* (writing `tmux.conf`). Keyfluent teaches using bindings, not authoring them.
- A full tmux or terminal emulator. The simulator models what the cards need, nothing more.

**Anti-goals** (failure even if it technically works)
- It becomes a flashcard quiz where you recall key *names*, or pick from options.
- It intercepts or swallows keys the learner needs while practicing (e.g. `Esc`, `Ctrl-b`).
- A beginner can't tell what to do on first launch, or what went wrong after a miss.
- Grading feels unfair: a correct answer marked wrong because the learner took another valid route.
- It feels like homework. Long sessions, guilt mechanics or nagging.
- Any network access.

## 3. Constraints

- **Rust + egui/eframe**, cross-platform: macOS first (universal arm64 + x86_64), plus Windows and Linux.
- **Development mirrors PCOReader** (`~/dev/pco_reader`, see §10): a Cargo workspace, a core
  crate with no UI, all builds and tests in a dev container on trantor (`bin/dev …`), and
  headless UI snapshot tests. Samuel builds the Mac app on his Mac from the GitHub repo.
- trantor is headless. GUI checks on trantor happen through offscreen snapshot tests
  (`egui_kittest`), and hands-on checks on the Mac.
- MIT license. Dependencies limited to permissive licenses, enforced by `cargo deny`.
- **No network.** `cargo deny` bans network crates (as in PCOReader). The macOS sandbox is
  built without the network entitlement.
- The OS keeps some shortcuts no app receives (macOS: `Cmd-Q`, `Cmd-Tab`, `Cmd-Space`,
  `Ctrl-←/→` for Spaces; Windows: `Win-…`, `Alt-Tab`, `Ctrl-Alt-Del`). Stock tmux and Vim
  mostly avoid these. Cards whose binding collides with an OS shortcut on the current platform
  are flagged (see §6.6).

## 4. Approaches considered

| Approach | Trade-off | Outcome |
|---|---|---|
| Browser app on trantor (simulated tmux in a web page, Tailscale access) | Zero install, but the browser reserves keys and Option combinations on a Mac are hard to read | Rejected by Samuel |
| Terminal (TUI) app | Real terminal keys, but a poor fit for beginners and for preferences and progress screens; and a TUI *inside* tmux can't see tmux's own prefix | Rejected |
| Real tmux embedded in the app (terminal widget + pty) | Authentic, but grading by state and timing is harder, needs a pty per platform (none on Windows) and is heavier | Rejected for v1 |
| **Native Rust + egui app with a simulated tool** | Full keyboard capture, a beginner-friendly GUI, and exact timing and state for grading. Each pack needs its own simulator model | **Chosen** (Samuel, 2026-11-25) |
| Iced or Tauri instead of egui | Iced is heavier. Tauri brings back a webview and some of its key quirks | egui chosen (Samuel) |

Possible later add-on (kept, not planned): a `keyfluent drill` command that runs inside a real
tmux, gives tasks and checks tmux's actual state.

## 5. Concepts

- **Pack:** one tool's lessons plus its simulator (`tmux`; later `vim`). A pack declares its
  **binding source** (stock or imported), its **categories** and its **cards**.
- **Card:** one skill. It has a goal, a start state, a canonical key sequence (the shortest
  stock way) and metadata. One card per binding *intent*, not per key.
- **Scenario:** a card whose goal needs several bindings in sequence. Unlocked once its
  component cards are solid.
- **Review:** one graded attempt at a card, logged with time, keys pressed and grade.
- **Simulator:** a model of the tool's state, plus a key → command dispatcher. It is pure
  logic in the core crate.

## 6. How it works

### 6.1 The tmux simulator (pack `tmux`)

The simulator implements **tmux commands**. **Bindings** map keys to those commands, exactly
as tmux does. So importing custom bindings only means replacing the key tables; the commands
stay the same.

- **State:** server → sessions (name, windows, current/last) → windows (index, name, layout
  tree of panes, active/last pane, zoom) → panes (id, size, fake content lines, mode). Plus:
  the client's attached session (or *detached*, showing a shell prompt outside tmux), paste
  buffers, the prefix-pending flag, the command prompt and the copy-mode state (cursor,
  selection start and type, scroll offset, search).
- **Key tables:** `root`, `prefix`, `copy-mode-vi` and `copy-mode` (emacs), parsed from
  `tmux list-keys` syntax. The stock tables ship as data in the pack. They are generated from
  stock tmux; `reference/tmux-3.0a-list-keys.txt` is a snapshot from trantor. The shipped
  stock tables are **tmux 3.4**: Ubuntu 24.04 ships it, trantor moves to 24.04 (task `pf-upgrade`),
  and it's close to Homebrew's. The v1 curriculum uses only bindings that are identical
  from 3.0a to 3.5. The builder documents the 3.0a → 3.4 differences in `reference/`.
- **Commands (v1 subset):** `split-window [-h|-v]`, `select-pane -L/-R/-U/-D/-t`,
  `select-pane -l` (`last-pane`), `resize-pane -Z` and arrow-resize, `kill-pane`,
  `break-pane`, `swap-pane -U/-D`, `rotate-window`, `next-layout`, `select-layout`,
  `display-panes`, `new-window`, `select-window -t N`, `next-window`, `previous-window`,
  `last-window`, `rename-window`, `kill-window`, `choose-tree` (sessions/windows picker),
  `new-session`, `rename-session`, `switch-client -n/-p/-l`, `detach-client`,
  `command-prompt`, `copy-mode`, `paste-buffer`, `choose-buffer`, `list-keys`
  (shown as a help overlay), `send-keys -X <copy-mode commands>`.
  **Copy-mode commands:** `cursor-*`, `next-word`, `previous-word`, `start-of-line`,
  `end-of-line`, `top-line`, `bottom-line`, `history-top`, `history-bottom`, `halfpage-up/down`,
  `page-up/down`, `search-forward/backward`, `search-again/reverse`, `begin-selection`,
  `select-line`, `rectangle-toggle`, `clear-selection`, `copy-selection-and-cancel`, `cancel`.
  Bindings whose command isn't implemented are listed as **"not simulated yet"** and produce
  no card.
- **Command prompt (`prefix :`):** accepts the implemented commands with their common
  flags and short aliases (`neww`, `splitw -h`, `new -s work`, `rename-window logs`…).
  Anything else shows tmux's own style of error line.
- **Outside tmux:** after a detach, the pane area becomes a fake shell. It accepts
  `tmux`, `tmux new -s NAME`, `tmux ls`, `tmux attach` / `tmux a [-t NAME]`. This is what
  the detach/attach cards need. Other input prints `command not found`.
- **Rendering model:** the core produces a cell grid (characters + style roles) for the
  terminal area and the status line. The UI only paints it. Pane borders, the active-pane
  highlight, the status bar with window list, the prefix indicator, copy-mode position indicator
  and selection highlight all follow real tmux closely enough that the habits carry over.
- **Fake pane content:** a few lines of plausible shell output per pane, from the card's start
  state. Copy-mode cards need real text to move through and select. Content is static. No
  program runs.

### 6.2 Key input

- During practice, **every key goes to the simulator** except one reserved **pause chord**.
  Default: `Ctrl-Shift-P` (Mac: `Cmd-Shift-P`); neither tmux nor Vim uses it by default.
  It can be changed in Preferences. Clicking the on-screen **Pause** button works too.
- egui key events are normalised to tmux key notation (`C-b`, `M-1`, `%`, `"`, `Space`,
  `Enter`, `Escape`, `Up`, `PPage`…):
  - Printable characters come from **text events**, so `%` and `"` work on any layout.
  - Control and Meta combinations come from **key events** with modifiers.
  - Mac Option is treated as Meta by default. A preference turns that off for people who type
    accented characters with Option. Mac terminals (Terminal.app, iTerm2) don't send Option as
    Meta unless set to, so **v1 has no `M-` cards** (layout presets `M-1…5`, `M-arrow` resize).
    They come later with a "your terminal must send Option as Meta" note.
- A **keystroke strip** under the terminal shows the last keys pressed, in the same notation.
  Beginners can see whether the prefix "took".

### 6.3 Cards and the learning flow

**Card file format.** TOML files in `packs/tmux/cards/*.toml`, one file per category, compiled
into the binary. Each card has:
- `id`, `category`, `intent` (the goal sentence, e.g. "Split this pane side by side")
- `start` (a named start-state fixture plus overrides)
- `goal`: a predicate over simulator state, e.g. `active_window.panes == 2 && layout == "h"`,
  or `copy_buffer == "error: disk full"`. Written in a small typed form, not free code.
- `canonical` (key sequence in the stock tables, e.g. `["C-b", "%"]`)
- `teach` (one or two plain sentences shown when the card is introduced)
- optional `requires` (card ids that must be learned first)

Scenarios use the same format, with `components = [card ids]`.

**Introducing a new card** (not graded):
1. **Show:** the intent, the goal picture, the keys spelled out, and the `teach` text.
2. **Guided:** do it with the keys still visible.
3. **Recall:** do it again from the intent alone.

The first graded review comes in the same session, after a few other cards, so it tests real
recall.

**Session flow.** The **Today** screen shows reviews due, new cards available, and a big
**Start** button (`Enter` works too). A session interleaves due reviews with new cards. Due
reviews come first; new cards slot in every few reviews. It ends when nothing is due and no
new cards remain, or whenever the learner pauses and quits. There is no timer.

**New-card limit:** off by default (Samuel). **Preferences → Daily new cards:** a number, or
"no limit".

**Curriculum order (tmux pack v1)**, matching where Samuel is:
1. **Basics:** what the prefix is; split side by side / top-bottom; move between panes (arrows,
   `o`, `;`); zoom; close a pane; new window; next/previous/numbered window; rename window; close window.
2. **Copy mode (vi keys):** enter copy mode (`[`, `PgUp`); move (`h j k l`, `w b`, `0 $`, `g G`,
   `C-u C-d`); search (`/ ? n N`); select (`Space`, `V` line, `v` block toggle); copy (`Enter`);
   leave (`q`); paste (`prefix ]`).
3. **Sessions:** detach / attach; new session (command prompt and shell); session picker (`s`,
   `w`); rename session (`$`); next/previous/last session (`( ) L`).
4. **Layout and more:** break pane to window (`!`), swap panes (`{ }`), cycle layouts
   (`Space`), show pane numbers (`q`), resize (`C-arrows`), command prompt basics, list keys (`?`).

Every binding in this list is checked against the stock tables when the pack is built.

### 6.4 Grading

An attempt starts when the card appears and ends when the goal predicate becomes true or the
attempt fails.

- **Any route that reaches the goal is correct.** After success, if the learner's route was
  longer than the canonical one, the app shows "Shortest: `C-b` `%`" as a hint.
- **Keys vs typed text.** A canonical route is a list of **steps**: binding keys (`C-b`, `%`)
  and **text entry** (typing into a tmux prompt, the command prompt, a search, or the fake
  shell: `logs`, `splitw -h`, `tmux a -t work`). Clearing a pre-filled prompt (`,` shows `#W`)
  and answering `confirm-before` with `y` are steps too.
- **Fail conditions:**
  - more than `binding_keys + slack` **binding keys** without reaching the goal (slack defaults
    to 3). Keys typed while a prompt, search or shell line is open don't count. So a
    command-prompt route (`C-b :` then `splitw -h` `Enter`) costs 2 binding keys plus free typing,
    and is never failed for length;
  - the learner asks for the answer (**Show me**, an on-screen button or the pause menu; no
    keyboard key, since every key belongs to the simulator);
  - a timeout of 30 s.
- **After a fail:** the start state resets, the canonical keys are shown and animated, and the
  learner does it once with them visible. That retry isn't graded. The card comes back later in
  the session.
- **Speed target per card:** `target = reaction + per_key × binding_keys + per_char × typed_chars`,
  defaults 1.2 s, 0.4 s and 0.15 s, computed from the canonical route. A slower alternate route,
  such as the command prompt instead of a binding, is correct but usually grades Hard. That's
  the intended nudge toward the binding. After 50 reviews, the reaction term adapts to the learner's own median, so slow
  typists aren't punished forever. It can be adjusted in Preferences ("speed expectations:
  relaxed / normal / brisk").
- **FSRS rating:**
  - **Again:** failed, or used the hint
  - **Hard:** correct, but slower than 2 × target, or more than 2 extra keys
  - **Good:** correct, within 2 × target
  - **Easy:** correct, faster than target, with no more binding keys than the canonical route

### 6.5 Scheduling

- **FSRS** with default parameters and desired retention 0.9, through the `rs-fsrs` crate
  (scheduler only; no ML dependencies). The builder confirms its license and API. Optimising
  parameters from the learner's own history is a later feature.
- **Scenario unlock:** a scenario becomes available when every component card has FSRS
  stability ≥ 7 days. The threshold is adjustable in Preferences.
- **Wild mode** (Preferences: Off / Sometimes / Always; default **Sometimes**, applied only to
  cards with stability ≥ 14 days): the goal picture is hidden and only the intent sentence
  shows. Closer to real use.

### 6.6 Screens

All screens are fully keyboard-operable when not practicing, and work with the mouse too.
Layout is designed for laptop and desktop screens (minimum window 1000 × 700). Visual design
comes from `ui-ux:ux-plan` (§9).

1. **Today:** pack switcher (only tmux in v1), due/new counts, Start, streak calendar strip.
2. **Practice:**
   - top: the intent sentence, large;
   - centre: the simulated terminal (cell grid, monospace);
   - right or below: the goal picture, a miniature of the target state (hidden in wild mode);
   - bottom: the keystroke strip, a progress dot row for the session, the Pause button.
   - After each card: a short result flash (grade, time vs target, shortest route if different).
     It advances automatically after ~1 s, or immediately on `Enter`.
3. **Pause menu** (pause chord): Resume, Show me, Skip card, End session.
4. **Progress:**
   - a mastery heatmap of every binding, grouped by category, colored by FSRS stability band
     (new / learning / young / mature);
   - per-card detail: recent times vs target, review history, next due;
   - a 30-day calendar of practice days and reviews;
   - a 14-day forecast of due reviews.
   - Charts follow the `dataviz` skill rules.
5. **Preferences:**
   - **Learning:** daily new cards (default no limit), wild mode, speed expectations, scenario
     unlock threshold.
   - **Keys:** copy-mode keys (**vi** default; emacs available), Mac Option as Meta, pause chord.
   - **Bindings:** stock, or **Import custom bindings** (paste or open a `tmux list-keys`
     output). Imports are validated and show a summary: changed, new and unsupported bindings.
     Cards follow the imported keys (§6.7).
   - **Appearance:** light / dark / follow system.
   - **Data:** export progress (JSON), import progress, reset progress (with confirmation).
6. **First launch:** a 60-second guided intro. What tmux is in one sentence, what the prefix
   is, then straight into the first card. No account, no setup questions.
7. **OS-shortcut warning:** cards whose keys collide with a shortcut reserved on the current
   platform show a badge and a one-line explanation, and are left out of sessions by default.

### 6.7 Custom bindings

- Import parses `bind-key [-r] [-T table] key command…` lines into key tables. It replaces the
  stock tables for the tables it contains.
- Cards are tied to **commands and goals**, not keys. With imported bindings, a card's
  canonical keys are recomputed from the new tables. A card whose command has no binding
  becomes "unbound — reachable via the command prompt" and is dropped from sessions.
- Commands the simulator doesn't implement are listed as unsupported in the import summary.
- If the prefix itself changes (e.g. `C-a`), the app says so plainly on the Today screen.
- Review history is kept per card id. FSRS state resets for cards whose keys changed, after a
  confirmation that says how many will reset.

### 6.8 Storage

- Location: the platform app-data directory (via the `directories` crate), e.g.
  `~/Library/Application Support/Keyfluent/`.
- Files: `prefs.json`, `progress.json` (FSRS state per pack and card), `reviews.jsonl`
  (append-only log), `bindings/<pack>.txt` (imported bindings). Written atomically (write
  temp, then rename).
- Schema versioned (`"schema": 1`) with migrations in the core.
- No other writes, no network, no logs containing anything but app errors.

## 7. Architecture

Cargo workspace, mirroring PCOReader:

```
crates/
  kf-core/     pure logic, no UI: key notation + parser, list-keys parser, pack/card model and
               loader, goal predicates, grading, FSRS wrapper, session builder, storage,
               preferences. Pack-agnostic traits (Simulator, Pack) live here.
  kf-tmux/     the tmux pack: simulator state, commands, stock key tables (generated data),
               cell-grid renderer, card files (include_str!).
  kf-app/      eframe/egui app "Keyfluent": screens, input normalisation, painting the cell
               grid, preferences UI. Binary name `keyfluent`.
  (later) kf-vim/
```

- `Simulator` trait: `reset(start)`, `key(KeyEvent) -> Outcome`, `grid() -> CellGrid`,
  `goal_met(&Goal) -> bool`, `describe_goal(&Goal) -> CellGrid` (for the goal picture).
- The core is fully unit-tested. Each tmux command is tested against the expected state, and
  every card is tested so that its canonical keys reach its goal from its start state. A test
  fails any card whose canonical keys don't work: this guards the content.
- UI tested with `egui_kittest` snapshot tests, headless in the dev container.

## 8. Accessibility and beginners

- Plain language everywhere. "Split this pane side by side", not "split-window -h".
- Key notation is shown as keycaps (`Ctrl` `b`, then `%`), with the tmux notation available on
  hover, because beginners won't know `C-b`.
- Colour is never the only signal: grades show a word plus an icon.
- WCAG 2.2 AA contrast in both themes. Text scales with the OS setting, plus an in-app zoom
  (`Cmd/Ctrl +/-` outside practice).
- Reduced-motion preference respected for the result flash and key animations.

## 9. UI/UX process

- `ui-ux:ux-plan` before any UI code: a discovery brief, 2–3 directions, **Samuel picks**,
  then `DESIGN.md` + `tokens.json`.
- `ui-ux:brand-identity` for the icon and wordmark, when packaging comes.
- `ui-ux:ux-build` to build screens from `DESIGN.md`.
- `ui-ux:ux-audit` before v0.1. `ui-ux:ux-refine` for its findings.

## 10. Development, builds, releases (mirrors PCOReader)

- **Toolchain container** `keyfluent-dev` (Rust stable + clippy, rustfmt, cargo-deny,
  egui Linux build deps, mesa for offscreen rendering, mingw-w64 + NSIS for Windows),
  run with `bin/dev <cmd>`. No R, unlike PCOReader.
- **Lints:** `unsafe_code = "forbid"`; clippy `unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing` warn.
- **`deny.toml`:** the PCOReader license allowlist and network-crate bans, adapted.
- **macOS:** `scripts/build-macos.sh` builds an unsigned universal `Keyfluent.app`
  (ad-hoc signed, App Sandbox without the network entitlement). Samuel pulls the repo on his
  Mac and runs it. First launch: right-click → Open.
- **Mac verification:** `scripts/mac/verify.sh` builds, runs tests and takes screenshots,
  writing a report. `--publish` pushes the report to branch `mac-results`. A Mac Claude Code
  session follows `MAC_AGENT.md`, as in PCOReader.
- **Windows:** `bin/dev scripts/build-windows.sh` cross-compiles an installer (NSIS) and a
  portable zip, with the same no-networking import check as PCOReader.
- **Linux:** `bin/dev cargo build --release`. AppImage later.
- **CI (my call):** a GitHub Actions workflow on push and PR: `fmt`, `clippy`, `test`,
  `cargo deny check`, on Linux. PCOReader has none, but a public open-source repo benefits from
  it. Release binaries stay manual for now.
- **Docs in repo:** `README.md` (what, screenshots, install, unsigned-app note),
  `CONTRIBUTING.md` (how to add cards and packs), `PLAN.md`, `STATUS.md`, `CLAUDE.md`.

## 11. Decided by Samuel (2026-11-25)

Name Keyfluent; general keyboard-shortcut trainer (tmux first, Vim later); Rust + egui
native app; beginner-friendly GUI for preferences and progress; intent → keys as the core;
automatic grading with speed as the fluency measure; FSRS; no new-card limit by default (a
preference); scenarios after simple cards; stock bindings by default, custom import in
preferences; no timer; no reminders; progress page; wild mode; one computer, local storage;
vi copy-mode keys; Mac builds done by Samuel on the Mac from the GitHub repo; unsigned for now;
MIT, public repo; PCOReader's development and build approach.

## 12. Calls I made (open to change)

- Pause chord `Ctrl/Cmd-Shift-P`; "Show me" hint via button and pause menu.
- Grading thresholds (slack 3 keys, 30 s timeout, target 1.2 s + 0.4 s/key, 2× for Hard).
- Unaided first review in the same session after introduction.
- Wild mode default **Sometimes**, only for cards with stability ≥ 14 days.
- Scenario unlock at stability ≥ 7 days.
- `rs-fsrs` rather than `fsrs` (which brings in an ML framework).
- JSON files rather than SQLite.
- GitHub Actions CI for tests/lint/deny.
- Curriculum order in §6.3.

- Card steps split into binding keys and typed text; only binding keys count toward the
  failure budget; per-character speed term (§6.4).
- Stock tables from tmux 3.4; curriculum limited to bindings unchanged across 3.0a–3.5; no `M-`
  cards in v1.
- **Milestones:** the first build Samuel uses is the end of P3 (Today + Practice + FSRS with
  the Basics cards, on the Mac). P4 adds copy mode, then sessions. Progress, Preferences,
  import and wild mode (P5) and scenarios (P6) follow. Windows packaging and the `ux-audit`
  gate belong to the v0.1 release (P7). Linux runs from `cargo` throughout.
- Repo public from the first commit: `samuelkordik/keyfluent`.
- Samuel's real setup (2026-11-25): tmux 3.0a on trantor, no `tmux.conf` until today,
  `~/.tmux.conf` now sets `mode-keys vi`; stock prefix `C-b`.

## 13. Left to the builder

- The exact goal-predicate syntax and fixture format.
- Fake pane content text.
- Cell-grid renderer details and font choice (bundled monospace font under OFL/permissive license).
- Reconciling the stock key tables with tmux 3.5 and documenting differences from 3.0a.
- egui version, and key-event details per platform (verified on the Mac in the first Mac round).
