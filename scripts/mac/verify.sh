#!/usr/bin/env bash
# Test and build the macOS app, and write a report.
#
#   scripts/mac/verify.sh                    run everything → out/mac/<date>-<commit>/
#   scripts/mac/verify.sh --publish          …then push the report to branch `mac-results`
#   scripts/mac/verify.sh --publish-run DIR  push an existing run folder (e.g. after adding
#                                            hands-on results to report.md), nothing else
#   --allow-dirty                            run even with local changes (never with --publish)
#
# One command, no prompts. Steps: check tools → cargo test --workspace → build the app
# (scripts/build-macos.sh). Published: only .md/.txt/.log/.png files from the run folder, with
# paths of this Mac replaced by <repo> and ~.
set -uo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT" || exit 1

die() { echo "verify: $*" >&2; exit 1; }
[[ "$(uname)" == "Darwin" ]] || die "run this on macOS"

PUBLISH=0 ALLOW_DIRTY=0 PUBLISH_RUN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --publish) PUBLISH=1 ;;
    --allow-dirty) ALLOW_DIRTY=1 ;;
    --publish-run)
      [[ $# -ge 2 && -n "$2" ]] || die "--publish-run needs a run folder, e.g. --publish-run out/mac/<date>-<commit>"
      PUBLISH_RUN="$2"; shift ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) die "unknown option $1" ;;
  esac
  shift
done

export PATH="$HOME/.cargo/bin:$PATH"

sanitize() { sed -e "s#$ROOT#<repo>#g" -e "s#$HOME#~#g"; }

# ---------------------------------------------------------------- publish
publish() {
  local run="${1%/}"
  [[ -d "$run" ]] || die "no such run folder: $run"
  run=$(cd "$run" && pwd -P) || die "cannot resolve $run"
  case "$run" in
    "$ROOT"/out/mac/[0-9]*) ;;
    *) die "refusing to publish '$1': only out/mac/<run> folders" ;;
  esac
  [[ -f "$run/report.md" ]] || die "$run has no report.md"
  local name base wt
  name=$(basename "$run")
  base=$(mktemp -d)
  wt="$base/mac-results"
  git worktree prune
  if git fetch -q origin mac-results 2>/dev/null; then
    git worktree add -q -B mac-results "$wt" origin/mac-results || die "worktree failed"
  elif git show-ref -q --verify refs/heads/mac-results; then
    git worktree add -q "$wt" mac-results || die "worktree failed"
  else
    git worktree add -q --detach "$wt" || die "worktree failed"
    (cd "$wt" && git checkout -q --orphan mac-results && git rm -rqf . >/dev/null 2>&1; true)
  fi
  local dest="$wt/runs/$name"
  mkdir -p "$dest"
  # Allow-list: reports, logs and screenshots only (no app, no binaries).
  local big
  big=$(cd "$run" && find . -type f \( -name '*.md' -o -name '*.txt' -o -name '*.log' -o -name '*.png' \) \
      -size +5M -print)
  if [[ -n "$big" ]]; then
    echo "verify: WARNING: not published (5 MB or larger):" >&2
    printf '%s\n' "$big" | while read -r f; do echo "  $f" >&2; done
  fi
  (cd "$run" && find . -type f \( -name '*.md' -o -name '*.txt' -o -name '*.log' -o -name '*.png' \) \
      -size -5M -print) |
    while read -r f; do
      mkdir -p "$dest/$(dirname "$f")"
      case "$f" in
        *.png) cp "$run/$f" "$dest/$f" ;;
        *) sanitize < "$run/$f" > "$dest/$f" ;;
      esac
    done
  { echo "# Latest Mac run: [$name](runs/$name/report.md)"; echo; cat "$dest/report.md"; } > "$wt/LATEST.md"
  (
    cd "$wt" || exit 1
    git add -A LATEST.md "runs/$name"
    git commit -q -m "Mac results $name" >/dev/null || echo "verify: nothing new to publish"
    local delay=2
    for _ in 1 2 3 4 5; do
      git push -q -u origin mac-results && exit 0
      sleep "$delay"; delay=$((delay * 2))
    done
    exit 1
  )
  local rc=$?
  git worktree remove --force "$wt" >/dev/null 2>&1
  rm -rf "$base"
  [[ $rc -eq 0 ]] || die "push to mac-results failed"
  echo "Published $run to branch mac-results (runs/$name)."
}

if [[ -n "$PUBLISH_RUN" ]]; then
  publish "$PUBLISH_RUN"
  exit 0
fi

if [[ -n "$(git status --porcelain)" ]]; then
  if [[ $ALLOW_DIRTY -eq 0 || $PUBLISH -eq 1 ]]; then
    git status --short
    die "the working tree has local changes; commit or stash them first"
  fi
fi

SHA=$(git rev-parse --short HEAD)
BRANCH=$(git branch --show-current)
STAMP=$(date +%Y%m%d-%H%M%S)
RUN="out/mac/$STAMP-$SHA"
LOG="$RUN/build.log"
mkdir -p "$RUN/hands-on"
: > "$LOG"

STEPS=()
BLOCKED=0
# step NAME CMD… — run, log and time one step. Later steps are skipped once the tools check fails.
step() {
  local name=$1; shift
  if [[ $BLOCKED -eq 1 ]]; then STEPS+=("$name|skipped|"); return 1; fi
  echo "== $name" | tee -a "$LOG"
  local t0=$SECONDS
  "$@" >> "$LOG" 2>&1
  local rc=$?
  local dt=$((SECONDS - t0))
  if [[ $rc -eq 0 ]]; then
    STEPS+=("$name|ok|${dt}s")
  else
    STEPS+=("$name|**FAILED** (exit $rc)|${dt}s")
    echo "   failed (exit $rc); see $LOG"
  fi
  return $rc
}

# shellcheck disable=SC2317  # run indirectly through step
check_tools() {
  local missing=()
  xcrun --sdk macosx --show-sdk-path >/dev/null 2>&1 || missing+=("Xcode command line tools: xcode-select --install")
  if ! command -v cargo >/dev/null; then
    missing+=("Rust: https://rustup.rs")
  else
    local ver
    ver=$(rustc --version 2>/dev/null | sed -n 's/^rustc \([0-9]*\)\.\([0-9]*\).*/\1 \2/p')
    # shellcheck disable=SC2086  # intentional split into major/minor
    set -- $ver
    if [[ $# -ne 2 ]] || [[ $1 -lt 1 || ( $1 -eq 1 && $2 -lt 95 ) ]]; then
      missing+=("Rust 1.95 or newer (found: $(rustc --version 2>&1)): rustup update stable")
    fi
  fi
  local t
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target list --installed 2>/dev/null | grep -qx "$t" \
      || missing+=("Rust target: rustup target add $t")
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    printf 'missing: %s\n' "${missing[@]}"
    return 1
  fi
}

# shellcheck disable=SC2317  # run indirectly through step
build_app() {
  scripts/build-macos.sh || return 1
  local app=dist/Keyfluent.app
  [[ -d "$app" ]] || return 1
  {
    echo "lipo: $(lipo -info "$app/Contents/MacOS/Keyfluent" 2>&1 | sed 's/.*are: //')"
    echo "entitlements: $(codesign -d --entitlements - "$app" 2>/dev/null | grep -o 'com.apple.security[a-z.-]*' | sort -u | tr '\n' ' ')"
    echo "signature: $(codesign --verify --deep --strict "$app" 2>&1 && echo ok)"
    echo "size: $(du -sh "$app" | cut -f1)"
  } > "$RUN/app.txt"
}

step "Tools" check_tools || BLOCKED=1
step "Tests (cargo test --workspace)" cargo test --workspace
step "Build app (universal)" build_app

# ---------------------------------------------------------------- report
grep -E 'error(\[|:)|FAILED|panicked|test result: FAILED|missing:' "$LOG" \
  | sanitize | head -150 > "$RUN/failures.txt"
grep -E '^warning' "$LOG" | sanitize | sort -u > "$RUN/warnings.txt"
tail -n 200 "$LOG" | sanitize > "$RUN/build-tail.log"
TESTS=$(grep -E '^test result:' "$LOG" | sed 's/^/- /')

{
  echo "# Mac verify $STAMP"
  echo
  echo "- Commit: \`$SHA\` on \`$BRANCH\` — $(git log -1 --format=%s)"
  echo "- macOS: $(sw_vers -productVersion) ($(sw_vers -buildVersion)), $(uname -m)"
  echo "- Xcode CLT: $(xcode-select -p 2>/dev/null)"
  echo "- Rust: $(rustc --version 2>/dev/null)"
  echo "- Cargo: $(cargo --version 2>/dev/null)"
  echo "- Targets: $(rustup target list --installed 2>/dev/null | tr '\n' ' ')"
  echo
  echo "## Steps"
  echo
  echo "| Step | Result | Time |"
  echo "|---|---|---|"
  for s in "${STEPS[@]}"; do echo "| ${s//|/ | } |"; done
  echo
  echo "## Tests"
  echo
  echo "${TESTS:-No test summary in the log.}"
  echo
  echo "## Warnings ($(wc -l < "$RUN/warnings.txt" | tr -d ' '))"
  echo
  if [[ -s "$RUN/warnings.txt" ]]; then echo '```'; head -40 "$RUN/warnings.txt"; echo '```'; else echo "None."; fi
  echo
  echo "## Failures"
  echo
  if [[ -s "$RUN/failures.txt" ]]; then echo '```'; head -60 "$RUN/failures.txt"; echo '```'; else echo "None."; fi
  echo
  echo "## App"
  echo
  if [[ -f "$RUN/app.txt" ]]; then sed 's/^/- /' "$RUN/app.txt"; else echo "Not built."; fi
  echo
  echo "## Key probe (hands-on)"
  echo
  echo "_Not yet recorded. Add it as described in MAC_AGENT.md, then run \`scripts/mac/verify.sh --publish-run $RUN\`._"
} | sanitize > "$RUN/report.md"

echo
cat "$RUN/report.md"
echo
echo "Run folder: $RUN"

RESULT=0
for s in "${STEPS[@]}"; do [[ "$s" == *FAILED* ]] && RESULT=1; done
if [[ $PUBLISH -eq 1 ]]; then
  publish "$RUN"
fi
exit $RESULT
