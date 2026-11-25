# Mac verify 20261125-125216

- Commit: `487d328` on `main` — Fix final-review findings: Mac agent docs, verify.sh hardening, MSRV 1.95, probe zoom at creation
- macOS: 26.7.3 (25G342), x86_64
- Xcode CLT: /Applications/Xcode.app/Contents/Developer
- Rust: rustc 1.99.1 (58cf8f9c3 2026-10-13)
- Cargo: cargo 1.99.1 (5f94df478 2026-08-27)
- Targets: aarch64-apple-darwin x86_64-apple-darwin 

## Steps

| Step | Result | Time |
|---|---|---|
| Tools | ok | 1s |
| Tests (cargo test --workspace) | ok | 23s |
| Build app (universal) | ok | 247s |

## Tests

- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
- test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

## Warnings (0)

None.

## Failures

None.

## App

- lipo: x86_64 arm64 
- entitlements: com.apple.security.app-sandbox 
- signature: ok
- size:  24M

## Key probe (hands-on)

Layout: **U.S.** Keys pressed one at a time, in this order, on a fresh launch (`--key-probe`).
Keyboard is a compact one without a PgUp key, so PgUp was not tested. Probe lists newest first;
rows below are in press order. Screenshots: `hands-on/probe.png` (full sequence), `hands-on/probe-option1.png` (Option-1 retest).

| Key pressed | Probe shows | Notes |
|---|---|---|
| `Ctrl-b` | ControlLeft down; `Key B` down/up `mods=C`; ControlLeft up | OK. No `Text` event. |
| `%` | ShiftLeft down; `Key Num5` down `mods=S`; `Text "%"`; Num5 up; ShiftLeft up | OK. Logical key is `Num5`, text carries `%`. |
| `"` | ShiftLeft down; `Key Quote` down `mods=S`; `Text "\""`; Quote up; ShiftLeft up | OK. Logical key `Quote`, text `"`. |
| `Shift-Up` | ShiftLeft down; `Key ArrowUp` down/up `mods=S`; ShiftLeft up | OK. |
| `Ctrl-Up` | ControlLeft down/up only, **no ArrowUp** | Swallowed by macOS (Mission Control default). |
| (unexpected) | `Key F5` down/up `mods=-` | Appeared between Ctrl-Up and Esc in both attempts; not one of the keys pressed. Source unknown (possibly the compact keyboard or a system shortcut side effect). |
| `Esc` | `Key Escape` down/up `mods=-` | OK. |
| `Option-1` | AltLeft down; `Key Num1` down `mods=M`, `physical=Some(Num1)`; `Text "¡"`; AltLeft up; Num1 up `mods=-` | OK (re-tested on a fresh launch, screenshot `hands-on/probe-option1.png`). Produces text `¡`, so matching must use the key + modifiers, not the text. In the first pass Samuel mistyped Option-i instead; that showed `Key I` `mods=M` with a text event, as expected for Option-i. |
| `PgUp` | — | Not tested (no PgUp on keyboard). |
| `Space` | `Key Space` down; `Text " "`; Space up `mods=-` | OK. |
| `Enter` | `Key Enter` down/up `mods=-` | OK. No `Text` event. |
| `Cmd-Shift-P` | SuperLeft down; ShiftLeft down `mods=Cmd`; `Key P` down/up `mods=S+Cmd`; ShiftLeft up; SuperLeft up | OK. No `Text` event. |

## Findings

- **Option+digit is fine.** `Option-1` gives `Num1` + `mods=M` (physical `Num1`) plus a `Text "¡"`
  event. Cards for `M-1`..`M-9` should match on the key event and ignore the text.
- **Ctrl-Up never reaches the app** (Mission Control). Cards that use `C-Up` need a note or an
  alternative binding on macOS.
- **Stray F5** appeared after Ctrl-Up in both runs; unexplained, recorded as observed.
- Modifier-press events carry the modifier set *before* the change (e.g. ShiftLeft down shows
  `mods=-`, ShiftLeft up shows `mods=S`); expected for egui but matters for matching logic.
