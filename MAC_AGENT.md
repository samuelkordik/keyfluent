# Mac agent instructions

You are a Claude Code session on Samuel's Mac. Development happens in another session on a Linux
server, which cannot build or run macOS code. Your job is to **build, test and observe** Keyfluent
on this Mac, then **report back through git**.

## Scope (hard rules)

- **Branches.** Pull `main` only. Push **only** to branch `mac-results`, and only through
  `scripts/mac/verify.sh --publish` or `--publish-run`. Never push to `main` or any other branch.
  Never force-push.
- **Code.** Don't edit tracked files, commit to `main`, or "fix" anything, even a one-line fix.
  The other session owns the code. Put suggested fixes in your report as diffs or prose.
- **Paths.** Reports must not contain paths outside the repository (home folder names, other
  projects) or anything from other apps. `verify.sh` replaces this Mac's repo and home paths with
  `<repo>` and `~`; do the same in anything you write yourself.
- **No network** beyond `git fetch`/`git pull`/`git push` to this repository's `origin`, and the
  `rustup target add` installs below when a target is missing. Keyfluent itself never uses the
  network (it is sandboxed without the network entitlement).
- **System changes.** Allowed: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`,
  deleting `out/mac/` and `dist/`. Ask Samuel first for anything else (`sudo`, system settings,
  other installs).

## Allowed commands

`git status|fetch|pull|switch|log|diff|show|worktree` (on the branches above),
`scripts/mac/verify.sh` with its options, `scripts/build-macos.sh`, `cargo build|test` (the
workspace), `open`/`killall` for the built app, `screencapture`, `sw_vers`, `codesign -d|--verify`,
`lipo -info`, `plutil -p` (on files in the build), `ls`, `cat`, `grep`, `find`, `du`, `sed` (to
sanitize your own report).

## A round, step by step

1. `git fetch origin && git switch main && git pull --ff-only`. If `git status` shows local
   changes, stop and report them (don't stash or discard).
2. `scripts/mac/verify.sh --publish` (run it **outside the Claude Code sandbox**: cargo writes to
   `~/.cargo`). It runs `cargo test --workspace`, builds `dist/Keyfluent.app`, writes
   `out/mac/<date>-<commit>/report.md` and pushes it to `mac-results`. First run: about 10
   minutes, later 2–3. It prints `report.md` at the end; note the run folder.
3. If a step failed, **diagnose** (read-only): read the end of `build.log` in the run folder and
   the failing source files. Write the error, file and line, your best explanation and a suggested
   fix in the report. List **every** `error:` line, not only the first. Don't change code.
4. **Key probe check (this round).** Run `open dist/Keyfluent.app --args --key-probe`. The key
   probe window shows what the app receives for each key press. Press each of these, one at a
   time, and note exactly what the probe shows (key name, modifiers, text, anything odd, or
   "nothing shown"):

   `Ctrl-b`, `%`, `"`, `Shift-Up`, `Ctrl-Up`, `Esc`, `Option-1`, `PgUp`, `Space`, `Enter`,
   `Cmd-Shift-P`

   (`%` and `"` are typed with Shift on a US layout; say which layout is active.) Some of these
   may be swallowed by macOS (`Ctrl-Up` is Mission Control by default); if so, record that, not a
   guess. Save a screenshot of the probe after the last key as `<run>/hands-on/probe.png`
   (`screencapture -x`). Close the app with `killall Keyfluent`.
5. Replace the "Key probe (hands-on)" placeholder section in `<run>/report.md` with a table:
   `| Key pressed | Probe shows | Notes |`, one row per key above.
6. `scripts/mac/verify.sh --publish-run out/mac/<date>-<commit>` — pushes the updated report and
   screenshots to `mac-results` (allow-listed file types only: `.md`, `.txt`, `.log`, `.png`).
7. Tell Samuel in a few lines: pass/fail, the main probe findings, and that the results are
   published.

## Report format

`report.md` (written by `verify.sh`, edited by you) has: commit and tool versions, a steps table,
test summary, warnings, failures, app facts (architectures, entitlements, signature, size) and
the key probe table. Add a short "Findings" section at the end if anything surprised you.

## When something is off

Stop and report rather than improvising: dirty working tree, a missing tool you may not install,
the app failing to launch (record the exact message; unsigned apps may need right-click > Open the
first time, or System Settings > Privacy & Security > Open Anyway; tell Samuel before changing
anything).
