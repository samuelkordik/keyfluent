//! End-to-end practice sessions over the Basics pack: plan, run, grade,
//! schedule, store and summarise, wired the way the app (P3) will wire them.
//!
//! The wiring rules this file follows (P3 obligations):
//! - every served item is reported exactly once, right after it is done;
//! - intro steps report `None` and never reach `SummaryBuilder::add`;
//! - reviews pass `retry` from the item to `SummaryBuilder::add`;
//! - `plan` and `Session::new` get the same `now` and `introduced_today`;
//! - progress is never saved after `load_progress` returned an error.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Days, FixedOffset, TimeZone, Utc};
use kf_core::card::Card;
use kf_core::grade::{Attempt, CardShape, Fail, Grade, Graded, Speed, Target};
use kf_core::key::Key;
use kf_core::prefs::Prefs;
use kf_core::schedule::{CardProgress, Scheduler};
use kf_core::session::{introduced_today, plan, CardRef, IntroStep, Item, Plan, Session};
use kf_core::speed::{personal_target, Sample};
use kf_core::stats::{days_this_month, due_tomorrow, Summary, SummaryBuilder};
use kf_core::store::{Progress, ReviewRecord, Store};
use kf_tmux::goal::Goal;
use kf_tmux::{fixtures, pack, stock, Tmux};
use tempfile::TempDir;

const PACK: &str = "tmux";
/// Time between two key presses in these tests.
const KEY_GAP: Duration = Duration::from_millis(200);
/// Wall-clock time each served item takes, so records are spread out.
const ITEM_GAP: chrono::Duration = chrono::Duration::seconds(10);

fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 26, 15, 0, 0).unwrap()
}

fn utc0() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap()
}

fn shape(card: &Card<Goal>) -> CardShape {
    CardShape {
        binding_keys: card.binding_key_count(),
        typed_chars: card.typed_char_count(),
        practiced: pack::practiced_bindings(card),
    }
}

/// Drives a fresh simulator from the card's start fixture with `keys`, one
/// every [`KEY_GAP`], checking the goal after each key as the app does.
/// Stops at the first failure or once the goal is met; a route that runs out
/// of keys without meeting the goal panics (tests only give complete routes
/// or routes that fail).
fn attempt(card: &Card<Goal>, target: Duration, keys: &[Key]) -> Graded {
    let server = fixtures::get(&card.start).expect("start fixture");
    let mut tmux = Tmux::new(server, stock::tables().clone());
    let mut a = Attempt::new(shape(card), target);
    let mut at = Duration::ZERO;
    for k in keys {
        at += KEY_GAP;
        if let Some(why) = a.tick(at) {
            return a.fail(why, at);
        }
        let outcome = tmux.key_at(k, at);
        if let Some(why) = a.record(&outcome, at) {
            return a.fail(why, at);
        }
        if card.goal.met(tmux.server()) {
            return a.succeed(at);
        }
    }
    panic!("{}: keys ran out before the goal or a failure", card.id);
}

/// The app's state for one pack while a session runs.
struct Practice {
    store: Store,
    cards: Vec<Card<Goal>>,
    refs: Vec<CardRef>,
    prefs: Prefs,
    progress: Progress,
    history: Vec<Sample>,
    scheduler: Scheduler,
    summary: SummaryBuilder,
    /// Wall clock: advances [`ITEM_GAP`] per served item.
    clock: DateTime<Utc>,
}

impl Practice {
    fn open(dir: &TempDir, now: DateTime<Utc>) -> Practice {
        let store = Store::at(dir.path().to_path_buf());
        // Never save after a load error: here it must load cleanly.
        let loaded = store.load_progress(now).expect("progress loads");
        assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
        let reviews = store.load_reviews();
        assert!(reviews.notes.is_empty(), "{:?}", reviews.notes);
        let cards = pack::basics().unwrap();
        let refs = cards.iter().map(CardRef::from_card).collect();
        Practice {
            store,
            cards,
            refs,
            prefs: Prefs {
                daily_new_cards: None,
                ..Prefs::default()
            },
            progress: loaded.value,
            history: reviews.value.iter().map(Sample::from).collect(),
            scheduler: Scheduler::new(),
            summary: SummaryBuilder::new(),
            clock: now,
        }
    }

    fn pack_progress(&self) -> BTreeMap<String, CardProgress> {
        self.progress.packs.get(PACK).cloned().unwrap_or_default()
    }

    fn introduced_today(&self, now: DateTime<Utc>) -> u32 {
        introduced_today(&self.store.load_reviews().value, PACK, now, utc0())
    }

    fn plan(&self, now: DateTime<Utc>) -> Plan {
        let today = self.introduced_today(now);
        plan(&self.refs, &self.pack_progress(), &self.prefs, today, now)
    }

    /// The Today screen's counts and a session built from the same inputs.
    fn begin(&self, now: DateTime<Utc>) -> (Plan, Session) {
        let today = self.introduced_today(now);
        let progress = self.pack_progress();
        let p = plan(&self.refs, &progress, &self.prefs, today, now);
        let s = Session::new(&self.refs, &progress, &self.prefs, today, now);
        (p, s)
    }

    fn card(&self, id: &str) -> &Card<Goal> {
        self.cards
            .iter()
            .find(|c| c.id == id)
            .expect("card in pack")
    }

    /// The target for `card` from the learner's history and speed.
    fn target(&self, card: &Card<Goal>) -> Duration {
        personal_target(self.prefs.speed, &self.history)
            .for_card(card.binding_key_count(), card.typed_char_count())
    }

    /// Applies a graded review: schedule, log, save, history, summary.
    fn apply(&mut self, card: &str, wild: bool, retry: bool, graded: &Graded) {
        let at = self.clock;
        let before = self.progress.packs.get(PACK).and_then(|m| m.get(card));
        let (next, new_best) = self.scheduler.review(before, graded, at);
        self.progress
            .packs
            .entry(PACK.to_string())
            .or_default()
            .insert(card.to_string(), next);
        let record = ReviewRecord {
            at,
            pack: PACK.to_string(),
            card: card.to_string(),
            grade: graded.grade,
            time_ms: ms(graded.time),
            target_ms: ms(graded.target),
            binding_keys: u32::try_from(graded.binding_keys).unwrap(),
            typed_chars: u32::try_from(graded.typed_chars).unwrap(),
            canonical_keys: u32::try_from(graded.canonical_keys).unwrap(),
            canonical_chars: u32::try_from(graded.canonical_chars).unwrap(),
            fail: graded.fail,
            capped: graded.capped,
            wild,
            intro: false,
        };
        self.store.append_review(&record).unwrap();
        self.store.save_progress(&self.progress).unwrap();
        self.history.push(Sample::from(graded));
        self.summary.add(card, graded, new_best, retry);
    }

    /// Serves `item`: intro steps are ungraded; reviews are driven with the
    /// keys `keys_for` picks. Reports the item exactly once and returns the
    /// review's grade.
    fn serve(
        &mut self,
        session: &mut Session,
        item: &Item,
        keys_for: impl Fn(&Card<Goal>, bool) -> Vec<Key>,
    ) -> Option<Graded> {
        self.clock += ITEM_GAP;
        match item {
            Item::Intro { .. } => {
                session.report(item, None);
                None
            }
            Item::Review { card, wild, retry } => {
                let c = self.card(card).clone();
                let graded = attempt(&c, self.target(&c), &keys_for(&c, *retry));
                self.apply(card, *wild, *retry, &graded);
                session.report(item, Some(&graded));
                Some(graded)
            }
        }
    }

    /// The session summary: reviews due tomorrow are [`due_tomorrow`] over
    /// the pack's current cards (ruling 14); new cards waiting are those
    /// tomorrow's daily cap allows.
    fn finish(self, session_time: Duration) -> Summary {
        let due = due_tomorrow(&self.refs, &self.pack_progress(), self.clock, utc0());
        let tomorrow = self.clock.checked_add_days(Days::new(1)).unwrap();
        let waiting = u32::try_from(self.plan(tomorrow).new_available).unwrap();
        self.summary.finish(session_time, due, waiting)
    }
}

fn canonical(card: &Card<Goal>, _retry: bool) -> Vec<Key> {
    pack::canonical_keys(card)
}

/// The grade the spec's formula gives a correct, canonical-route attempt
/// taking `time` against `target` (no extra keys, practiced keys covered).
fn expected_grade(time: Duration, target: Duration) -> Grade {
    if time > target * 2 {
        Grade::Hard
    } else if time < target {
        Grade::Easy
    } else {
        Grade::Good
    }
}

#[test]
fn full_session_round_trip() {
    let dir = TempDir::new().unwrap();
    let now = start();
    let mut app = Practice::open(&dir, now);
    let ids: Vec<String> = app.cards.iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids.len(), 17);

    let (today, mut session) = app.begin(now);
    assert_eq!(
        today,
        Plan {
            due: 0,
            new_available: 17
        }
    );

    let mut intros: BTreeMap<String, Vec<IntroStep>> = BTreeMap::new();
    let mut reviews: BTreeMap<String, Vec<Graded>> = BTreeMap::new();
    while let Some(item) = session.next() {
        match &item {
            Item::Intro { card, step } => intros.entry(card.clone()).or_default().push(*step),
            Item::Review { card, wild, retry } => {
                assert!(
                    !wild && !retry,
                    "{card}: fresh cards are never wild or retried"
                );
                assert!(
                    intros.get(card).is_some_and(|s| s.len() == 3),
                    "{card}: reviewed before its intro finished"
                );
            }
        }
        let graded = app.serve(&mut session, &item, canonical);
        if let (Item::Review { card, .. }, Some(g)) = (&item, graded) {
            reviews.entry(card.clone()).or_default().push(g);
        }
    }

    // Every card introduced (Show, Guided, Recall) and reviewed once.
    let full_intro = vec![IntroStep::Show, IntroStep::Guided, IntroStep::Recall];
    for id in &ids {
        assert_eq!(intros.get(id), Some(&full_intro), "{id}");
        let g = reviews.get(id).unwrap_or_else(|| panic!("{id}: no review"));
        assert_eq!(g.len(), 1, "{id}");
        let g = &g[0];
        // Canonical keys at 200 ms each: the formula says Easy for every
        // card (the slowest, window-rename, is 8 keys = 1.6 s against 3.4 s).
        assert_eq!(g.grade, expected_grade(g.time, g.target), "{id}: {g:?}");
        assert_eq!(g.grade, Grade::Easy, "{id}: {g:?}");
        assert!(!g.capped && g.fail.is_none(), "{id}: {g:?}");
        assert_eq!(g.binding_keys, g.canonical_keys, "{id}");
        assert_eq!(g.typed_chars, g.canonical_chars, "{id}");
    }
    let window_rename = &reviews["window-rename"][0];
    assert_eq!(
        (window_rename.time, window_rename.target),
        (
            Duration::from_millis(1600),
            Target::standard(Speed::Normal).for_card(4, 4)
        )
    );

    // Progress was saved and reloads equal; the log has 17 records.
    let reloaded = Store::at(dir.path().to_path_buf())
        .load_progress(now)
        .unwrap();
    assert!(reloaded.notes.is_empty());
    assert_eq!(reloaded.value, app.progress);
    let log = app.store.load_reviews();
    assert!(log.notes.is_empty());
    assert_eq!(log.value.len(), 17);
    assert!(log.value.iter().all(|r| r.grade == Grade::Easy && !r.intro));

    // Nothing left right now (ruling 13: the first Easy is scheduled as Good,
    // a short learning step); everything comes back within 30 days.
    assert_eq!(
        app.plan(now),
        Plan {
            due: 0,
            new_available: 0
        }
    );
    assert_eq!(app.introduced_today(now), 17);
    let later = now.checked_add_days(Days::new(30)).unwrap();
    assert_eq!(
        app.plan(later),
        Plan {
            due: 17,
            new_available: 0
        }
    );
    assert_eq!(days_this_month(&log.value, now, utc0()), 1);
    assert_eq!(
        due_tomorrow(&app.refs, &app.pack_progress(), now, utc0()),
        17
    );

    let summary = app.finish(Duration::from_secs(600));
    assert_eq!(summary.cards, 17);
    assert_eq!(summary.per_grade, [0, 0, 0, 17]);
    assert_eq!(summary.best_combo, 17);
    assert_eq!(summary.new_bests.len(), 17);
    assert_eq!(
        (summary.next_reviews_tomorrow, summary.new_waiting),
        (17, 0)
    );
}

#[test]
fn wrong_keys_fail_and_retry() {
    let dir = TempDir::new().unwrap();
    let now = start();
    let mut app = Practice::open(&dir, now);
    let (_, mut session) = app.begin(now);

    // On split-h's first review, the learner presses `C-b "` (a vertical
    // split) and then six unbound keys; the rest go right.
    let wrong: Vec<Key> = ["C-b", "\"", "F5", "F6", "F7", "F8", "F9", "F10"]
        .iter()
        .map(|k| k.parse().unwrap())
        .collect();
    let keys_for = move |card: &Card<Goal>, retry: bool| {
        if card.id == "split-h" && !retry {
            wrong.clone()
        } else {
            pack::canonical_keys(card)
        }
    };

    let mut served = Vec::new();
    let mut split_h = Vec::new();
    while let Some(item) = session.next() {
        let graded = app.serve(&mut session, &item, &keys_for);
        if let (Item::Review { card, retry, .. }, Some(g)) = (&item, graded) {
            if card == "split-h" {
                split_h.push((served.len(), *retry, g));
            }
        }
        served.push(item);
    }

    assert_eq!(split_h.len(), 2, "{split_h:?}");
    let (first_at, first_retry, first) = &split_h[0];
    assert!(!first_retry);
    assert_eq!(first.grade, Grade::Again);
    assert_eq!(first.fail, Some(Fail::TooManyKeys));
    // Budget: 2 canonical + 3 slack; the 6th binding key fails.
    assert_eq!(first.binding_keys, 6);
    assert_eq!(first.time, Duration::from_millis(1200));

    let (retry_at, retry, second) = &split_h[1];
    assert!(retry, "the second split-h review is the retry");
    assert!(
        retry_at - first_at > 3,
        "retry gap: {first_at} -> {retry_at}"
    );
    assert_eq!(second.grade, Grade::Easy);
    assert!(matches!(
        &served[*retry_at],
        Item::Review {
            wild: false,
            retry: true,
            ..
        }
    ));

    // The log keeps both attempts; the summary counts the card once.
    let log = app.store.load_reviews().value;
    let grades: Vec<Grade> = log
        .iter()
        .filter(|r| r.card == "split-h")
        .map(|r| r.grade)
        .collect();
    assert_eq!(grades, [Grade::Again, Grade::Easy]);
    let summary = app.finish(Duration::from_secs(600));
    assert_eq!(summary.cards, 17);
    assert_eq!(summary.per_grade, [1, 0, 0, 17]);
}
