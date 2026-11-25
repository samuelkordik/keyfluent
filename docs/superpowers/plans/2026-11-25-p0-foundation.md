# P0 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A buildable, linted, license-checked Cargo workspace with a dev container, CI, Mac build scripts, and a key-probe window that proves which key events reach an egui app on each platform.

**Architecture:** Mirrors `~/dev/pco_reader`: Cargo workspace (`kf-core`, `kf-tmux`, `kf-app`), all commands run through `bin/dev` in container `keyfluent-dev`, headless `egui_kittest` tests on trantor, unsigned universal `.app` built on Samuel's Mac.

**Tech Stack:** Rust stable (edition 2021, `rust-version = "1.85"`), eframe/egui 0.37 (as PCOReader; take the newest 0.x at execution if both compile), egui_kittest (wgpu + snapshot), cargo-deny, Docker Compose, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-25-keyfluent-design.md` (§3, §7, §10)

## Global Constraints

- MIT license; workspace `license = "MIT"`, `authors = ["Samuel Kordik"]`, `repository = "https://github.com/samuelkordik/keyfluent"`.
- `[workspace.lints.rust] unsafe_code = "forbid"`; clippy `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` = `"warn"`.
- No network crates. `deny.toml` copies PCOReader's license allowlist and `[bans] deny` list (drop the MPL-2.0/UniFFI line).
- Binary name `keyfluent`; app display name `Keyfluent`; bundle id `com.samuelkordik.keyfluent`.
- macOS: universal (arm64 + x86_64), ad-hoc signed, App Sandbox **without** network entitlement, `LSMinimumSystemVersion` 11.0.
- Container name `keyfluent-dev`, image `keyfluent-dev:latest`; no R, no Python reference tools.

## Review Focus

1. Key events that egui swallows or reports oddly on macOS (`Ctrl-b` as text? `%` with Shift? Option combos producing `¡`): the key probe must show *both* the `Key` events and `Text` events with modifiers, unfiltered.
2. Running `bin/dev` before the image exists: first run builds it; expect `docker compose up -d dev` to succeed from a clean checkout.
3. Headless wgpu not available (CI runner without lavapipe): snapshot tests are `#[ignore]` with a reason, so `cargo test` passes in CI; the container runs them with `--include-ignored`.
4. `cargo deny` failing on a transitive dependency's license (egui fonts are OFL-1.1/Ubuntu-font-1.0): allowlist already covers them; CI must run deny so a new crate can't slip in.
5. `build-macos.sh` run on Linux: exits with "Run this on macOS." (status 1), never half-builds.

---

### Task 1: Dev container

**Files:**
- Create: `docker/Dockerfile`, `docker-compose.yml`, `bin/dev`, `.devcontainer/devcontainer.json`

**Interfaces:**
- Produces: `bin/dev <cmd>` runs `<cmd>` in `/workspace` inside `keyfluent-dev` (same script as `~/dev/pco_reader/bin/dev`, with the comment lines adapted).

- [ ] **Step 1:** Write `docker/Dockerfile` from `~/dev/pco_reader/docker/Dockerfile`: base `ubuntu:24.04` (not rocker); keep the build tools, egui/eframe Linux deps, `mesa-vulkan-drivers`, `mingw-w64 nsis`, `shellcheck`; drop R, pandoc, python, libxml/curl/git2 dev packages; rustup stable + `rustfmt clippy rust-src rust-analyzer`, target `x86_64-pc-windows-gnu`; `cargo install --locked cargo-deny`; same non-root `dev` user block (uid/gid 1000).
- [ ] **Step 2:** Write `docker-compose.yml` (service `dev`, image/container names above, volumes `.:/workspace` and `cargo-registry:/opt/rust/cargo/registry`), `bin/dev`, and `.devcontainer/devcontainer.json` (drop the R/Python extensions).
- [ ] **Step 3:** Verify: `bin/dev cargo --version && bin/dev cargo deny --version && bin/dev shellcheck --version`. Expected: three version lines.
- [ ] **Step 4:** Commit `chore: dev container`.

### Task 2: Workspace, crates, lints

**Files:**
- Create: `Cargo.toml`, `crates/kf-core/{Cargo.toml,src/lib.rs}`, `crates/kf-tmux/{Cargo.toml,src/lib.rs}`, `crates/kf-app/{Cargo.toml,src/lib.rs,src/main.rs}`, `rust-toolchain.toml` (channel `stable`)

**Interfaces:**
- Produces: `kf_core::VERSION: &str` (= `env!("CARGO_PKG_VERSION")`); `kf-tmux` depends on `kf-core`; `kf-app` depends on both, `[[bin]] name = "keyfluent"`. All crates `[lints] workspace = true`. `[profile.release] lto = "thin"`, `codegen-units = 1`.

- [ ] **Step 1:** Write test `crates/kf-core/src/lib.rs`: `#[test] fn version_is_set() { assert!(!VERSION.is_empty()); }`.
- [ ] **Step 2:** Run `bin/dev cargo test -p kf-core` → FAIL (crate missing).
- [ ] **Step 3:** Create the workspace and crates as above. `kf-app/src/main.rs` opens an eframe window titled "Keyfluent" (minimum size 1000 × 700, per spec §6.6) showing the version.
- [ ] **Step 4:** Run `bin/dev cargo test --workspace && bin/dev cargo clippy --workspace --all-targets -- -D warnings && bin/dev cargo fmt --check` → all pass.
- [ ] **Step 5:** Commit `chore: workspace and crate skeleton`.

### Task 3: Key-probe window + headless test

**Files:**
- Create: `crates/kf-app/src/probe.rs`, `crates/kf-app/tests/probe.rs`
- Modify: `crates/kf-app/src/lib.rs`, `crates/kf-app/src/main.rs`

**Interfaces:**
- Produces: `pub struct ProbeApp { log: Vec<String> }` implementing `eframe::App`; `pub fn describe(event: &egui::Event) -> Option<String>`, which returns `"Key <key:?> pressed=<bool> mods=<C|M|S|Cmd flags>"` for `Event::Key`, `"Text <string:?>"` for `Event::Text`, `None` for others. `main.rs` runs `ProbeApp` when started with `--key-probe` (otherwise the placeholder window). The probe shows the last 40 lines and a one-line instruction: "Press keys. Every event egui receives is listed below."

- [ ] **Step 1:** Test in `tests/probe.rs`:
```rust
#[test]
fn describes_ctrl_b_and_percent() {
    let ctrl_b = egui::Event::Key { key: egui::Key::B, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::CTRL };
    assert_eq!(kf_app::probe::describe(&ctrl_b).as_deref(), Some("Key B pressed=true mods=C"));
    assert_eq!(kf_app::probe::describe(&egui::Event::Text("%".into())).as_deref(), Some("Text \"%\""));
}
#[test]
fn probe_logs_typed_text() { /* Harness::builder().build_eframe(|_| ProbeApp::default()); h.key_press(egui::Key::A) + type "%"; run_steps(2); assert the label containing "Text \"%\"" exists via get_by_label_contains */ }
```
  (Adjust the `Event::Key` fields to the egui version's exact struct.)
- [ ] **Step 2:** `bin/dev cargo test -p kf-app` → FAIL.
- [ ] **Step 3:** Implement `probe.rs`; read events from `ctx.input(|i| i.raw.events.clone())` before widgets consume them, and switch off egui's own keyboard shortcuts (zoom, Tab focus) so nothing is eaten.
- [ ] **Step 4:** `bin/dev cargo test -p kf-app` → PASS.
- [ ] **Step 5:** Commit `feat: key-probe window`.

### Task 4: License and network checks, CI

**Files:**
- Create: `deny.toml`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces: CI job `check` on `push` and `pull_request`: `ubuntu-latest`, install the Linux egui deps from the Dockerfile, then `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `EmbarkStudios/cargo-deny-action` (`check`).

- [ ] **Step 1:** Write `deny.toml` (see Global Constraints) with comments adapted to Keyfluent.
- [ ] **Step 2:** Verify the ban works: temporarily add `ureq = "2"` to `kf-core`, run `bin/dev cargo deny check bans` → FAIL naming `ureq`; remove it; rerun → `bans ok`.
- [ ] **Step 3:** `bin/dev cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
- [ ] **Step 4:** Write `ci.yml`. Its first real check is the push in Task 6.
- [ ] **Step 5:** Commit `ci: fmt, clippy, tests, cargo-deny`.

### Task 5: macOS build and Mac round scripts

**Files:**
- Create: `scripts/build-macos.sh`, `scripts/mac/verify.sh`, `MAC_AGENT.md`

**Interfaces:**
- Produces: `scripts/build-macos.sh` → `dist/Keyfluent.app`, adapted from `~/dev/pco_reader/scripts/build-macos.sh` with the Quick Look extension, document types and icon steps removed (icon comes in P7; until then no `CFBundleIconFile`). `scripts/mac/verify.sh [--publish] [--allow-dirty]` → runs `cargo test --workspace`, builds the app, writes `out/mac/<date>-<commit>/report.md` with tool versions, test summary and build result; `--publish` pushes only `.md/.txt/.log/.png` from that folder to branch `mac-results` (same mechanism as PCOReader's script, simplified: no xcodegen/xcodebuild). `MAC_AGENT.md`: instructions for Samuel's Mac Claude Code session: pull `main`, run `scripts/mac/verify.sh --publish`, then for this round launch `open dist/Keyfluent.app --args --key-probe` and record, in `report.md`, what the probe shows for: `Ctrl-b`, `%`, `"`, `Shift-Up`, `Ctrl-Up`, `Esc`, `Option-1`, `PgUp`, `Space`, `Enter`, `Cmd-Shift-P`.

- [ ] **Step 1:** Write the three files.
- [ ] **Step 2:** `bin/dev shellcheck scripts/build-macos.sh scripts/mac/verify.sh` → no findings.
- [ ] **Step 3:** `bash scripts/build-macos.sh; echo $?` on trantor → prints "Run this on macOS." and `1`.
- [ ] **Step 4:** Commit `build: macOS app and Mac round scripts`.

### Task 6: Publish and hand over

- [ ] **Step 1:** Update `README.md` (Build section: `bin/dev cargo run` is headless, so on a desktop: `cargo run -p kf-app`; macOS: `scripts/build-macos.sh`, then right-click → Open on first launch) and add `STATUS.md` ("P0 done; Mac round 1 pending: key probe").
- [ ] **Step 2:** `git push`; confirm the CI run is green: `gh run watch --exit-status`.
- [ ] **Step 3:** Close dashboard task `kf-p0` (`~/dev/homelab/dashboard/instance/dash task status kf-p0 done`) and tell Samuel Mac round 1 is ready (`MAC_AGENT.md`).
