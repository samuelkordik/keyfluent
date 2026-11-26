# P2 Learning Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The pure-logic learning engine the app will drive: grade an attempt from key outcomes and time, schedule cards with FSRS, build and run a practice session (intro flow, interleaving, retries), keep personal bests, combo and session summary, and store everything on disk safely.

**Architecture:** New modules in `kf-core` (pack-agnostic, no UI): `grade`, `speed`, `schedule`, `prefs`, `store`, `session`, `stats`. The app (P3) feeds `KeyOutcome`s and "goal met" from the pack's simulator into an `Attempt`, gets a `Graded`, and hands it to the `Session`. All time is injected (`DateTime<Utc>` for calendar time, `Duration` since attempt start for key timing), so everything is testable without a clock. A kf-tmux integration test runs a whole session headless.

**Tech Stack:** Rust; `rs-fsrs` 1.2 (feature `serde`; MIT via license file), `chrono` (serde), `serde_json`, `directories` (app-data dir); dev: `tempfile`.

**Spec:** `docs/superpowers/specs/2026-11-25-keyfluent-design.md` (§6.3 learning flow, §6.4 grading, §6.5 scheduling, §6.6 screens' data, §6.8 storage) and `DESIGN.md` §3/§7 (stars, NEW BEST, combo, session summary, days this month, mastery bands). P1 follow-ups: `docs/superpowers/notes/p1-followups.md` (P2 items).

## Global Constraints

- No panics on any input, including corrupt or hand-edited files on disk. Typed `thiserror` errors.
- No clock reads inside the engine: callers pass `now: DateTime<Utc>` and attempt-relative `Duration`s. Local calendar dates (days this month, "tomorrow") use a passed `chrono::FixedOffset` or `Tz`-free offset, so tests are deterministic.
- No network. New dependencies must pass `bin/dev cargo deny check` (add a `[[licenses.clarify]]` for `rs-fsrs` if its license file isn't detected).
- Grading values (spec §6.4): slack 3 binding keys; timeout 30 s; target = 1.2 s + 0.4 s × binding keys + 0.15 s × typed chars; Hard if time > 2 × target or binding keys > canonical + 2; Easy if time < target and binding keys ≤ canonical; Good otherwise; Again on fail or hint. Speed expectations multiply the target: relaxed × 1.5, normal × 1.0, brisk × 0.75. After 50 graded reviews the 1.2 s reaction term becomes the learner's median reaction (clamped 0.8–3.0 s).
- Ruling 16 (P1): a correct route whose bindings don't include all of the card's practiced bindings (table + key) is capped at Good, and the result names the practiced keys.
- Ruling 17 (P1): `KeyOutcome.text_entry == false` keys are binding keys; those with `ran == None` that a prompt ignored still count (they are wasted keys).
- Stars: Easy ★★★, Good ★★☆, Hard ★☆☆, Again ☆☆☆ (DESIGN §7).
- Mastery bands (DESIGN §7 Progress): new = never reviewed; learning = FSRS state Learning or Relearning; young = Review with stability < 21 days; mature = Review with stability ≥ 21 days.
- FSRS: desired retention 0.9, default weights, short-term scheduling on, fuzz off.
- Storage (spec §6.8): `prefs.json`, `progress.json` (`"schema": 1`), `reviews.jsonl` (append-only), atomic writes (temp file + rename). Unreadable files are moved aside (`<name>.bad-<unix-ts>`), never deleted, and the load reports it.
- Scenarios (`components` non-empty) are excluded from sessions until P6.

## Review Focus

1. **Corrupt or future-schema `progress.json`** (hand-edited, truncated by a crash, written by a newer version): load must not panic, must keep the bad file aside, and must say so; a future `schema` (> 1) is refused without overwriting it.
2. **Clock going backwards or a review "in the future"** (laptop clock changed, timezone travel): FSRS elapsed days must not go negative or panic; due-ness uses `max(0, …)`.
3. **A session where every card fails**: retries must not loop forever — each card is retried at most twice per session, then dropped to the next session.
4. **Deleting or renaming a card id between versions** (progress has ids the pack no longer has): ignored for sessions, kept in the file.
5. **Daily new-card limit across midnight and across app restarts**: "introduced today" is counted from the review log by local date, not from memory.

---

### Task 1: Grading an attempt

**Files:** Create `crates/kf-core/src/grade.rs`; modify `crates/kf-core/src/lib.rs`, `crates/kf-core/Cargo.toml`; modify `crates/kf-tmux/src/pack.rs` (add `practiced_bindings`).

**Interfaces:**
- Consumes: `KeyOutcome { text_entry, ran: Option<RanBinding> }`, `RanBinding { table, key, command }`, `Card::binding_key_count`, `Card::typed_char_count`.
- Produces (kf-core `grade`):
  - `pub enum Grade { Again, Hard, Good, Easy }` with `stars(self) -> u8` (0–3) and `label(self) -> &'static str` (`"AGAIN"`, `"HARD"`, `"GOOD"`, `"EASY"`).
  - `pub enum Speed { Relaxed, Normal, Brisk }` (serde, lowercase) with `factor(self) -> f64`.
  - `pub struct Target { pub reaction: Duration, pub per_key: Duration, pub per_char: Duration, pub factor: f64 }`, `Target::standard(speed: Speed) -> Target` (1.2 s, 0.4 s, 0.15 s), `Target::for_card(&self, binding_keys: usize, typed_chars: usize) -> Duration`.
  - `pub struct CardShape { pub binding_keys: usize, pub typed_chars: usize, pub practiced: Vec<RanBinding> }`.
  - `pub enum Fail { TooManyKeys, Timeout, Hint }`.
  - `pub struct Attempt` with `Attempt::new(shape: CardShape, target: Duration) -> Attempt`, `record(&mut self, outcome: &KeyOutcome, at: Duration) -> Option<Fail>` (returns `Some(TooManyKeys)` once binding keys exceed `shape.binding_keys + 3` without success), `tick(&mut self, at: Duration) -> Option<Fail>` (`Timeout` at 30 s), `hint(&mut self) -> Fail`, `succeed(self, at: Duration) -> Graded`, `fail(self, why: Fail, at: Duration) -> Graded`.
  - `pub struct Graded { pub grade: Grade, pub time: Duration, pub target: Duration, pub binding_keys: usize, pub typed_chars: usize, pub fail: Option<Fail>, pub capped: bool /* Ruling 16 */, pub practiced: Vec<RanBinding> }`.
- Produces (kf-tmux `pack`): `pub fn practiced_bindings(card: &Card<Goal>) -> Vec<RanBinding>`: replay the canonical keys from the start fixture with the stock tables and collect every `ran`.

- [ ] **Step 1: Failing tests** (`grade.rs`): target for 2 binding keys and 0 chars at Normal = 2.0 s, at Relaxed = 3.0 s; `succeed` at 1.5 s with 2 keys → Easy ★★★; at 2.5 s → Good; at 4.1 s → Hard; with 5 binding keys (canonical 2) at 1.0 s → Hard; with 3 keys at 1.0 s → Good (not Easy: more than canonical); `record` of a 6th binding key on a 2-key card → `Some(TooManyKeys)`; keys with `text_entry: true` never count; `tick(Duration::from_secs(30))` → `Some(Timeout)`; `fail(Hint, …)` → Again ☆☆☆; Ruling 16: a fast correct attempt whose recorded `ran` bindings don't include every `practiced` binding (matched by table and key) → Good with `capped == true`; one that includes them all → Easy, `capped == false`. kf-tmux: `practiced_bindings` of `split-h` is one binding, table `prefix`, key `%`, command `split-window`; of `window-rename` its first entry's command is `command-prompt`.
- [ ] **Step 2:** `bin/dev cargo test -p kf-core grade` → FAIL. **Step 3:** Implement. **Step 4:** → PASS; workspace tests, clippy `-D warnings`, fmt.
- [ ] **Step 5:** Commit `feat(core): grade attempts`.

### Task 2: Personal reaction time

**Files:** Create `crates/kf-core/src/speed.rs`.

**Interfaces:**
- Consumes: `Target`, `Graded`.
- Produces: `pub fn personal_target(speed: Speed, history: &[Graded]) -> Target` — with fewer than 50 successful graded reviews returns `Target::standard(speed)`; otherwise the reaction term is the median of `time − per_key × binding_keys − per_char × typed_chars` over the last 50 successful reviews (not Again), clamped to 0.8–3.0 s; `per_key`, `per_char` and `factor` unchanged.

- [ ] **Step 1: Failing tests:** 49 reviews → standard; 50 reviews each with 2 binding keys and time 2.3 s → reaction 1.5 s; reaction never below 0.8 s or above 3.0 s; Again reviews ignored; only the last 50 count.
- [ ] **Step 2–4:** fail → implement → pass (plus workspace checks). **Step 5:** Commit `feat(core): personal reaction time`.

### Task 3: Scheduling and mastery

**Files:** Create `crates/kf-core/src/schedule.rs`; add `rs-fsrs = { version = "1.2", features = ["serde"] }` and `chrono = { version = "0.4", features = ["serde"] }` to kf-core; `deny.toml` clarify if needed.

**Interfaces:**
- Produces:
  - `pub struct CardProgress { pub fsrs: rs_fsrs::Card, pub reviews: u32, pub best: Option<Duration>, pub last_times: Vec<Duration> /* newest last, max 10 */ }` (serde; `Duration` as milliseconds).
  - `pub enum Mastery { New, Learning, Young, Mature }` and `CardProgress::mastery(&self) -> Mastery`.
  - `pub struct Scheduler` with `Scheduler::new() -> Scheduler` (retention 0.9, short-term on, fuzz off) and `review(&self, progress: Option<&CardProgress>, graded: &Graded, now: DateTime<Utc>) -> (CardProgress, bool /* new best */)`. Maps Grade → `rs_fsrs::Rating`. A new best = success (not Again) faster than the previous best, or the first success. `last_times` keeps successful times only.
  - `CardProgress::is_due(&self, now: DateTime<Utc>) -> bool` and `due(&self) -> DateTime<Utc>`.

- [ ] **Step 1: Failing tests:** first review Good → mastery Learning, due in the future, `reviews == 1`, best set, new best true; a second, slower Good → new best false; Again → no best change; `last_times` capped at 10; repeated Easy reviews over simulated days (advance `now` to each due date) reach Mature (stability ≥ 21) within 8 reviews; a `now` earlier than `last_review` (Review Focus 2) neither panics nor produces a due date before `now`; serde round-trip of `CardProgress`.
- [ ] **Step 2–4:** fail → implement → pass; `bin/dev cargo deny check` passes. **Step 5:** Commit `feat(core): FSRS scheduling and mastery`.

### Task 4: Preferences and storage

**Files:** Create `crates/kf-core/src/prefs.rs`, `crates/kf-core/src/store.rs`; add `serde_json`, `directories`; dev-dep `tempfile`.

**Interfaces:**
- Produces:
  - `pub enum WildMode { Off, Sometimes, Always }` (default Sometimes); `pub enum CopyModeKeys { Vi, Emacs }` (default Vi).
  - `pub struct Prefs { pub daily_new_cards: Option<u32> /* None = no limit (default) */, pub wild_mode: WildMode, pub speed: Speed, pub scenario_unlock_days: u32 /* 7 */, pub copy_mode_keys: CopyModeKeys, pub option_as_meta: bool /* true */, pub pause_chord: String /* "C-S-p" */ }` with `Default`, `#[serde(default)]` on every field so older/newer files load.
  - `pub struct ReviewRecord { pub at: DateTime<Utc>, pub pack: String, pub card: String, pub grade: Grade, pub time_ms: u64, pub target_ms: u64, pub binding_keys: u32, pub typed_chars: u32, pub fail: Option<Fail>, pub capped: bool, pub wild: bool, pub intro: bool }` (serde).
  - `pub struct Progress { pub schema: u32, pub packs: BTreeMap<String, BTreeMap<String, CardProgress>> }`.
  - `pub struct Store { root: PathBuf }`: `Store::at(root: PathBuf) -> Store`, `Store::default_location() -> Option<Store>` (via `directories::ProjectDirs::from("com", "samuelkordik", "Keyfluent")`), `load_prefs(&self) -> Loaded<Prefs>`, `save_prefs(&self, &Prefs) -> Result<(), StoreError>`, `load_progress(&self) -> Result<Loaded<Progress>, StoreError>` (`Err(StoreError::FutureSchema(n))` for schema > 1, file untouched), `save_progress(&self, &Progress) -> Result<(), StoreError>` (atomic), `append_review(&self, &ReviewRecord) -> Result<(), StoreError>`, `load_reviews(&self) -> Loaded<Vec<ReviewRecord>>` (skips bad lines, counts them).
  - `pub struct Loaded<T> { pub value: T, pub notes: Vec<String> }` — notes like "progress.json was unreadable; kept as progress.json.bad-1764100000; starting fresh" or "skipped 2 unreadable lines in reviews.jsonl".

- [ ] **Step 1: Failing tests** (all in a `tempfile::TempDir`): missing files → defaults, no notes; save/load round-trips for prefs and progress; prefs JSON with an unknown field and a missing field loads; truncated `progress.json` → fresh progress, a note, and the original kept as `progress.json.bad-*`; `{"schema": 2, …}` → `FutureSchema(2)` and the file is unchanged; `save_progress` leaves no temp file behind and the previous content survives if the rename target directory is read-only (skip this case if running as root); `append_review` ×3 then a garbage line then 1 more → `load_reviews` returns 4 records and one note.
- [ ] **Step 2–4:** fail → implement → pass; deny check. **Step 5:** Commit `feat(core): preferences and storage`.

### Task 5: Session builder and runner

**Files:** Create `crates/kf-core/src/session.rs`.

**Interfaces:**
- Consumes: `Card<G>` metadata (id, requires, components), `Progress` for one pack, `Prefs`, `ReviewRecord`s, `Grade`, `Graded`, `CardProgress::is_due`.
- Produces:
  - `pub struct CardRef { pub id: String, pub requires: Vec<String>, pub is_scenario: bool }` and `CardRef::from_card<G>(&Card<G>) -> CardRef`.
  - `pub enum Step { Show, Guided, Recall }` (intro phases; Show = intent, goal, keys and teach on screen, learner does it with keys visible = Guided; Recall = from intent alone; none graded).
  - `pub enum Item { Intro { card: String, step: Step }, Review { card: String, wild: bool, retry: bool } }`.
  - `pub struct Plan { pub due: usize, pub new_available: usize }` and `pub fn plan(cards: &[CardRef], progress: &BTreeMap<String, CardProgress>, prefs: &Prefs, introduced_today: u32, now: DateTime<Utc>) -> Plan` (for the Today screen).
  - `pub struct Session` with `Session::new(cards: &[CardRef], progress: &BTreeMap<String, CardProgress>, prefs: &Prefs, introduced_today: u32, now: DateTime<Utc>) -> Session`, `next(&mut self) -> Option<Item>`, `report(&mut self, item: &Item, result: Option<&Graded>)` (Intro steps report `None`), `remaining(&self) -> usize`.
  - `pub fn introduced_today(reviews: &[ReviewRecord], pack: &str, now: DateTime<Utc>, offset: FixedOffset) -> u32` (distinct cards whose first-ever record for this pack has a local date equal to today's).
- Rules (spec §6.3): due reviews first, most overdue first; new cards in file order, only when all `requires` have at least one review, skipping scenarios and ids with progress; daily cap from `prefs.daily_new_cards` minus `introduced_today` (no cap when None); one new card slots in after every 3 reviews (all at once if there are no reviews); a new card runs Intro Show → Guided → Recall, then its first graded Review comes 2 items later (or at the end); an Again review comes back 3 items later with `retry: true`, at most twice per card per session (Review Focus 3); wild per prefs: Off never; Always for every review of a card with stability ≥ 14 days; Sometimes for those cards on alternate occurrences (deterministic: the 2nd, 4th… eligible review in the session). Unknown ids in progress are ignored (Review Focus 4).

- [ ] **Step 1: Failing tests:** empty progress + cards A and B where B requires A → the session is exactly `Intro A Show, Intro A Guided, Intro A Recall, Review A` (B waits for a persisted review of A, i.e. the next session); 5 due reviews + 2 new → order R R R (new 1 intro ×3) R R (new 2 intro ×3) with each new card's Review 2 items after its Recall; daily cap 1 with `introduced_today` 1 → no new cards; an Again on review X → X returns 3 items later with `retry: true`, and a third Again does not bring it back; scenarios never appear; `plan` counts match the session's distinct cards; `introduced_today` across a local midnight (offset −06:00) counts correctly (Review Focus 5); unknown id in progress ignored; wild Off/Always/Sometimes behaviour as specified.
- [ ] **Step 2–4:** fail → implement → pass. **Step 5:** Commit `feat(core): session builder and runner`.

### Task 6: Session stats

**Files:** Create `crates/kf-core/src/stats.rs`.

**Interfaces:**
- Produces:
  - `pub struct Combo { current: u32, best: u32 }` with `push(&mut self, g: Grade)` (any non-Again extends; Again resets current to 0 silently), `current()`, `best()`.
  - `pub struct Summary { pub cards: u32, pub time: Duration, pub best_combo: u32, pub per_grade: [u32; 4] /* Again, Hard, Good, Easy */, pub new_bests: Vec<(String, Duration)>, pub next_reviews_tomorrow: u32, pub new_waiting: u32 }` and `SummaryBuilder` with `add(&mut self, card: &str, graded: &Graded, new_best: bool)` and `finish(self, session_time: Duration, plan_tomorrow: &Plan) -> Summary` (intro steps and retries: retries count in per_grade and combo, not in `cards`).
  - `pub fn days_this_month(reviews: &[ReviewRecord], now: DateTime<Utc>, offset: FixedOffset) -> u32` (distinct local dates in the current local month).
  - `pub fn due_tomorrow(progress: &BTreeMap<String, CardProgress>, now: DateTime<Utc>, offset: FixedOffset) -> u32` (cards due by the end of the next local day).

- [ ] **Step 1: Failing tests:** combo Good Easy Hard Again Good → current 1, best 3; summary counts per grade and `cards` excludes retries; new bests listed in order; days_this_month with records on 3 local dates this month, 1 last month and 2 on the same day → 3, including one at 23:30 local that is the next day in UTC; due_tomorrow boundary at local midnight.
- [ ] **Step 2–4:** fail → implement → pass. **Step 5:** Commit `feat(core): combo, summary, days this month`.

### Task 7: End-to-end session test and docs

**Files:** Create `crates/kf-tmux/tests/session.rs`; modify `STATUS.md`, `docs/superpowers/notes/p1-followups.md` (tick the P2 items done).

**Interfaces:**
- Consumes everything above plus `kf_tmux::{Tmux, fixtures, stock, pack}`.

- [ ] **Step 1: Failing test** `full_session_round_trip`: with a `TempDir` store and `now` = 2026-11-26T15:00Z: build a session from the Basics cards (no progress, no cap); for each item, for Intro steps report `None`; for Reviews start an `Attempt` from the card's shape and `personal_target`, drive a fresh `Tmux` from the card's start fixture with its canonical keys at 200 ms intervals via `key_at`, `record` each outcome, check the goal, `succeed`, `Scheduler::review`, append the `ReviewRecord`, update progress, feed `Combo`/`SummaryBuilder`. Assert: every Basics card is introduced and reviewed once; all grades are Easy (canonical keys, fast); progress saved, reloaded equal; `plan` at the same `now` shows 0 new and 0 due; `plan` at `now + 30 days` shows 17 due; `days_this_month` = 1.
- [ ] **Step 2:** a second test `wrong_keys_fail_and_retry`: on `split-h`, press `C-b "` then six unbound keys → `TooManyKeys` → Again; the session brings it back with `retry: true`.
- [ ] **Step 3:** run → FAIL until wired; fix only test-side glue (engine changes go back to their task's module with a test). **Step 4:** PASS; workspace tests, clippy, fmt, deny.
- [ ] **Step 5:** Update STATUS.md (P2 done) and tick P2 items in p1-followups.md; commit `test: end-to-end practice session`.
