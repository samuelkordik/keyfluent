# Mac key probe, round 1 (2026-11-25)

Source: `mac-results` branch, run `20261125-125216-487d328` (macOS 26.7.3, x86_64, U.S. layout,
compact keyboard without PgUp). Probe = `keyfluent --key-probe` (egui 0.37).

| Key | What egui delivers |
|---|---|
| `Ctrl-b` | `Key B` mods=C; no Text |
| `%` | `Key Num5` mods=S, then `Text "%"` |
| `"` | `Key Quote` mods=S, then `Text "\""` |
| `Shift-Up` | `Key ArrowUp` mods=S |
| `Ctrl-Up` | **never arrives** (macOS Mission Control); app then gets a bare `Key F5`, no mods |
| `Esc` | `Key Escape` |
| `Option-1` | `Key Num1` mods=M, physical Num1, then `Text "¡"` |
| `Space` | `Key Space`, then `Text " "` |
| `Enter` | `Key Enter`; no Text |
| `Cmd-Shift-P` | `Key P` mods=S+Cmd; no Text |
| `PgUp` | not tested (no key; compact Mac keyboards send it with Fn-Up) |

Modifier keys arrive as their own Key events, carrying the modifier set from *before* the change.

## Rules for input normalisation (P3, `kf-app` → `kf_core::Key`)

1. **Printable characters without Ctrl/Meta come from `Text`.** Ignore the preceding `Key`
   event when it has only Shift (or nothing) and a printable Text follows (`%`, `"`, `Space`).
2. **Ctrl or Meta combinations come from the `Key` event** (+ physical key for Meta) and the
   following `Text` is ignored (`Option-1` → `M-1`, not `¡`).
3. **Named keys** (Enter, Escape, arrows, F-keys) come from `Key` events; Shift on a named key
   becomes `S-` (`S-Up`).
4. **Modifier-only Key events are ignored.**
5. **macOS reserves `Ctrl-arrow`** (Mission Control/Spaces). Cards whose canonical keys include
   `C-Up/Down/Left/Right` get the OS-shortcut flag on macOS (spec §6.6), and the grader ignores
   a bare `F5` that immediately follows a Ctrl press/release with no other key.
6. **PgUp on compact keyboards** is `Fn-Up`; the copy-mode card for `prefix PgUp` says so (P4).
