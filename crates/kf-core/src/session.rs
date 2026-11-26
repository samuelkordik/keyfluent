//! Building and running a practice session (spec §6.3).
//!
//! A session serves due reviews first (most overdue first) and slots one new
//! card in after every three reviews. A new card runs through its ungraded
//! intro (Show, Guided, Recall); its first graded review follows two items
//! later. An Again brings the card back three items later, at most twice per
//! card per session. The engine never reads the clock: callers pass `now`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};

use crate::card::Card;
use crate::grade::{Grade, Graded};
use crate::prefs::{Prefs, WildMode};
use crate::schedule::CardProgress;
use crate::store::ReviewRecord;

/// Reviews served between two new cards.
const REVIEWS_PER_NEW: usize = 3;
/// Items served between a new card's Recall step and its first review.
const FIRST_REVIEW_GAP: usize = 2;
/// Items served between an Again and its retry.
const RETRY_GAP: usize = 3;
/// Retries per card per session.
const MAX_RETRIES: u32 = 2;
/// Stability (days) from which a card's reviews may be wild.
const WILD_STABILITY: f64 = 14.0;

/// What the session needs to know about a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRef {
    pub id: String,
    pub requires: Vec<String>,
    /// Scenarios (cards with components) never appear in sessions.
    pub is_scenario: bool,
}

impl CardRef {
    pub fn from_card<G>(card: &Card<G>) -> CardRef {
        CardRef {
            id: card.id.clone(),
            requires: card.requires.clone(),
            is_scenario: !card.components.is_empty(),
        }
    }
}

/// The ungraded phases of introducing a new card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntroStep {
    /// The intent, goal, keys and teach text on screen.
    Show,
    /// The learner does it with the keys still visible.
    Guided,
    /// The learner does it from the intent alone.
    Recall,
}

/// One thing for the learner to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Intro {
        card: String,
        step: IntroStep,
    },
    Review {
        card: String,
        /// Hide the goal picture; show only the intent.
        wild: bool,
        /// This review repeats an Again from earlier in the session.
        retry: bool,
    },
}

/// What the Today screen shows before a session starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub due: usize,
    pub new_available: usize,
}

/// Count due reviews and new cards available now.
pub fn plan(
    cards: &[CardRef],
    progress: &BTreeMap<String, CardProgress>,
    prefs: &Prefs,
    introduced_today: u32,
    now: DateTime<Utc>,
) -> Plan {
    Plan {
        due: due_cards(cards, progress, now).len(),
        new_available: new_cards(cards, progress, prefs, introduced_today).len(),
    }
}

/// Due cards, most overdue first (file order breaks ties).
fn due_cards(
    cards: &[CardRef],
    progress: &BTreeMap<String, CardProgress>,
    now: DateTime<Utc>,
) -> Vec<String> {
    let mut due: Vec<(DateTime<Utc>, &str)> = cards
        .iter()
        .filter(|c| !c.is_scenario)
        .filter_map(|c| {
            let p = progress.get(&c.id)?;
            p.is_due(now).then_some((p.due(), c.id.as_str()))
        })
        .collect();
    due.sort_by_key(|(at, _)| *at);
    due.into_iter().map(|(_, id)| id.to_string()).collect()
}

/// New cards to introduce, in file order, within the daily cap.
fn new_cards(
    cards: &[CardRef],
    progress: &BTreeMap<String, CardProgress>,
    prefs: &Prefs,
    introduced_today: u32,
) -> Vec<String> {
    let cap = prefs
        .daily_new_cards
        .map(|n| usize::try_from(n.saturating_sub(introduced_today)).unwrap_or(usize::MAX));
    let reviewed = |id: &String| progress.get(id).is_some_and(|p| p.reviews >= 1);
    cards
        .iter()
        .filter(|c| !c.is_scenario && !progress.contains_key(&c.id))
        .filter(|c| c.requires.iter().all(reviewed))
        .take(cap.unwrap_or(usize::MAX))
        .map(|c| c.id.clone())
        .collect()
}

/// A review waiting to be served after some more items.
#[derive(Debug)]
struct Scheduled {
    /// Items still to serve before this one.
    after: usize,
    card: String,
    retry: bool,
}

/// A practice session in progress.
///
/// Spacing rules (controller rulings 10 and 11):
/// - An intro (Show, Guided, Recall) is never interrupted.
/// - A scheduled review (a new card's first review, or a retry after an
///   Again) waits for at least its gap (2 items after Recall, 3 after an
///   Again). It is served at the next free slot after that, or at the end
///   if nothing else is left. Slots can collide, for example when the gap
///   runs out during an intro or two reviews come due together. The gap is
///   then longer, never shorter.
/// - Retries are never wild and don't count toward Sometimes' alternation.
#[derive(Debug)]
pub struct Session {
    due: VecDeque<String>,
    new: VecDeque<String>,
    /// The new card being introduced and its next step. Show is served as
    /// the intro starts, so this only ever holds Guided or Recall.
    intro: Option<(String, IntroStep)>,
    /// Scheduled reviews, in the order they were scheduled.
    scheduled: Vec<Scheduled>,
    /// Reviews served since the last new card started.
    reviews_since_new: usize,
    /// Retries scheduled per card.
    retries: BTreeMap<String, u32>,
    /// Cards whose reviews may be wild (stability >= 14 days).
    wild_eligible: BTreeSet<String>,
    wild_mode: WildMode,
    /// Wild-eligible reviews served so far.
    wild_seen: u32,
}

impl Session {
    pub fn new(
        cards: &[CardRef],
        progress: &BTreeMap<String, CardProgress>,
        prefs: &Prefs,
        introduced_today: u32,
        now: DateTime<Utc>,
    ) -> Session {
        let due = due_cards(cards, progress, now);
        let wild_eligible = due
            .iter()
            .filter(|id| {
                progress
                    .get(*id)
                    .is_some_and(|p| p.fsrs.stability >= WILD_STABILITY)
            })
            .cloned()
            .collect();
        Session {
            due: due.into(),
            new: new_cards(cards, progress, prefs, introduced_today).into(),
            intro: None,
            scheduled: Vec::new(),
            reviews_since_new: 0,
            retries: BTreeMap::new(),
            wild_eligible,
            wild_mode: prefs.wild_mode,
            wild_seen: 0,
        }
    }

    /// The next item, or `None` when the session is over. Not an
    /// `Iterator`: what comes next depends on what was reported.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Item> {
        let item = self.pick()?;
        if let Item::Review { .. } = item {
            self.reviews_since_new = self.reviews_since_new.saturating_add(1);
        }
        Some(item)
    }

    fn pick(&mut self) -> Option<Item> {
        // An intro runs its three steps without interruption.
        if let Some((card, step)) = self.intro.take() {
            self.tick();
            match step {
                IntroStep::Show => {}
                IntroStep::Guided => self.intro = Some((card.clone(), IntroStep::Recall)),
                IntroStep::Recall => self.scheduled.push(Scheduled {
                    after: FIRST_REVIEW_GAP,
                    card: card.clone(),
                    retry: false,
                }),
            }
            return Some(Item::Intro { card, step });
        }
        if let Some(i) = self.scheduled.iter().position(|s| s.after == 0) {
            let s = self.scheduled.remove(i);
            self.tick();
            return Some(self.review(s.card, s.retry));
        }
        if !self.new.is_empty()
            && (self.reviews_since_new >= REVIEWS_PER_NEW || self.due.is_empty())
        {
            if let Some(card) = self.new.pop_front() {
                self.reviews_since_new = 0;
                self.intro = Some((card.clone(), IntroStep::Guided));
                self.tick();
                return Some(Item::Intro {
                    card,
                    step: IntroStep::Show,
                });
            }
        }
        if let Some(card) = self.due.pop_front() {
            self.tick();
            return Some(self.review(card, false));
        }
        // Only scheduled reviews are left: serve the soonest.
        let i =
            (0..self.scheduled.len()).min_by_key(|i| self.scheduled.get(*i).map(|s| s.after))?;
        let s = self.scheduled.remove(i);
        self.tick();
        Some(self.review(s.card, s.retry))
    }

    /// One item was served: the scheduled reviews come one step closer.
    fn tick(&mut self) {
        for s in &mut self.scheduled {
            s.after = s.after.saturating_sub(1);
        }
    }

    fn review(&mut self, card: String, retry: bool) -> Item {
        let wild = !retry
            && self.wild_eligible.contains(&card)
            && match self.wild_mode {
                WildMode::Off => false,
                WildMode::Always => true,
                WildMode::Sometimes => {
                    self.wild_seen = self.wild_seen.saturating_add(1);
                    self.wild_seen.is_multiple_of(2)
                }
            };
        Item::Review { card, wild, retry }
    }

    /// Report how an item went. Report each served item exactly once, right
    /// after it is done. Intro steps report `None`; a review reports its
    /// grade. An Again brings the card back three items later, at most
    /// twice per card per session.
    pub fn report(&mut self, item: &Item, result: Option<&Graded>) {
        let Item::Review { card, .. } = item else {
            return;
        };
        if result.map(|g| g.grade) != Some(Grade::Again) {
            return;
        }
        let retries = self.retries.entry(card.clone()).or_insert(0);
        if *retries >= MAX_RETRIES {
            return;
        }
        *retries += 1;
        self.scheduled.push(Scheduled {
            after: RETRY_GAP,
            card: card.clone(),
            retry: true,
        });
    }

    /// Items left to serve, not counting retries that may still come.
    pub fn remaining(&self) -> usize {
        // The stored step is Guided (2 steps left) or Recall (1 left).
        let intro = match &self.intro {
            Some((_, IntroStep::Guided)) => 2,
            Some(_) => 1,
            None => 0,
        };
        // A pending intro still owes its first review.
        let first_review = usize::from(self.intro.is_some());
        intro + first_review + self.scheduled.len() + self.due.len() + self.new.len() * 4
    }
}

/// Distinct cards of `pack` whose first-ever record falls on today's local
/// date (at `offset`).
pub fn introduced_today(
    reviews: &[ReviewRecord],
    pack: &str,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> u32 {
    let local = |at: DateTime<Utc>| -> NaiveDate { at.with_timezone(&offset).date_naive() };
    let mut first: BTreeMap<&str, DateTime<Utc>> = BTreeMap::new();
    for r in reviews.iter().filter(|r| r.pack == pack) {
        first
            .entry(r.card.as_str())
            .and_modify(|at| *at = (*at).min(r.at))
            .or_insert(r.at);
    }
    let today = local(now);
    let n = first.values().filter(|at| local(**at) == today).count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;
    use crate::schedule::new_card;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 26, 18, 0, 0).unwrap()
    }

    fn card(id: &str, requires: &[&str]) -> CardRef {
        CardRef {
            id: id.to_string(),
            requires: requires.iter().map(|s| s.to_string()).collect(),
            is_scenario: false,
        }
    }

    fn scenario(id: &str) -> CardRef {
        CardRef {
            is_scenario: true,
            ..card(id, &[])
        }
    }

    /// Progress due `hours_ago` hours before now (negative: in the future).
    fn prog(hours_ago: i64, stability: f64) -> CardProgress {
        let mut fsrs = new_card(now());
        fsrs.due = now() - Duration::hours(hours_ago);
        fsrs.stability = stability;
        CardProgress {
            fsrs,
            reviews: 1,
            best: None,
            last_times: Vec::new(),
        }
    }

    fn graded(grade: Grade) -> Graded {
        Graded {
            grade,
            time: std::time::Duration::from_secs(1),
            target: std::time::Duration::from_secs(3),
            binding_keys: 2,
            typed_chars: 0,
            canonical_keys: 2,
            canonical_chars: 0,
            fail: None,
            capped: false,
            practiced: Vec::new(),
        }
    }

    fn intro(id: &str, step: IntroStep) -> Item {
        Item::Intro {
            card: id.to_string(),
            step,
        }
    }

    fn rev(id: &str) -> Item {
        Item::Review {
            card: id.to_string(),
            wild: false,
            retry: false,
        }
    }

    fn retry(id: &str) -> Item {
        Item::Review {
            card: id.to_string(),
            wild: false,
            retry: true,
        }
    }

    fn intros(id: &str) -> Vec<Item> {
        vec![
            intro(id, IntroStep::Show),
            intro(id, IntroStep::Guided),
            intro(id, IntroStep::Recall),
        ]
    }

    /// Run a session to the end, grading every review Good.
    fn drain(s: &mut Session) -> Vec<Item> {
        let mut out = Vec::new();
        while let Some(item) = s.next() {
            let g = graded(Grade::Good);
            let result = matches!(item, Item::Review { .. }).then_some(&g);
            s.report(&item, result);
            out.push(item);
            assert!(out.len() < 1000, "session never ends");
        }
        out
    }

    fn session(
        cards: &[CardRef],
        progress: &BTreeMap<String, CardProgress>,
        prefs: &Prefs,
    ) -> Session {
        Session::new(cards, progress, prefs, 0, now())
    }

    #[test]
    fn from_card_marks_scenarios() {
        let src = r#"
[[card]]
id = "a"
category = "c"
intent = "i"
teach = "t"
start = "s"
goal = 1
canonical = ["C-b"]
requires = ["x"]

[[card]]
id = "sc"
category = "c"
intent = "i"
teach = "t"
start = "s"
goal = 1
canonical = ["C-b"]
components = ["a"]
"#;
        let cards = crate::card::load_cards::<u32>(src).unwrap();
        let a = CardRef::from_card(&cards[0]);
        assert_eq!(a.id, "a");
        assert_eq!(a.requires, vec!["x".to_string()]);
        assert!(!a.is_scenario);
        assert!(CardRef::from_card(&cards[1]).is_scenario);
    }

    #[test]
    fn requires_waits_for_a_persisted_review() {
        let cards = [card("A", &[]), card("B", &["A"])];
        let mut s = session(&cards, &BTreeMap::new(), &Prefs::default());
        assert_eq!(s.remaining(), 4);
        let mut expected = intros("A");
        expected.push(rev("A"));
        assert_eq!(drain(&mut s), expected);
        assert_eq!(s.remaining(), 0);
    }

    #[test]
    fn new_cards_slot_in_after_every_three_reviews() {
        let mut cards: Vec<CardRef> = (1..=5).map(|i| card(&format!("r{i}"), &[])).collect();
        cards.push(card("n1", &[]));
        cards.push(card("n2", &[]));
        // r1 most overdue, r5 least: file order is reversed to prove sorting.
        cards.reverse();
        let progress: BTreeMap<_, _> = (1..=5)
            .map(|i| (format!("r{i}"), prog(10 - i, 1.0)))
            .collect();
        let mut s = session(&cards, &progress, &Prefs::default());
        assert_eq!(s.remaining(), 5 + 2 * 4);
        let mut expected = vec![rev("r1"), rev("r2"), rev("r3")];
        // n2 comes before n1 in (reversed) file order.
        expected.extend(intros("n2"));
        expected.extend([rev("r4"), rev("r5"), rev("n2")]);
        expected.extend(intros("n1"));
        expected.push(rev("n1"));
        assert_eq!(drain(&mut s), expected);
    }

    #[test]
    fn first_review_comes_two_items_after_recall() {
        let mut cards: Vec<CardRef> = (1..=9).map(|i| card(&format!("r{i}"), &[])).collect();
        cards.push(card("n", &[]));
        let progress: BTreeMap<_, _> = (1..=9)
            .map(|i| (format!("r{i}"), prog(10 - i, 1.0)))
            .collect();
        let items = drain(&mut session(&cards, &progress, &Prefs::default()));
        let recall = items
            .iter()
            .position(|i| *i == intro("n", IntroStep::Recall))
            .unwrap();
        let review = items.iter().position(|i| *i == rev("n")).unwrap();
        assert_eq!(review, recall + 3);
        assert_eq!(items.len(), 9 + 4);
    }

    #[test]
    fn daily_cap_counts_cards_introduced_today() {
        let cards = [card("A", &[]), card("B", &[])];
        let prefs = Prefs {
            daily_new_cards: Some(1),
            ..Prefs::default()
        };
        let mut s = Session::new(&cards, &BTreeMap::new(), &prefs, 1, now());
        assert_eq!(s.next(), None);
        let p = plan(&cards, &BTreeMap::new(), &prefs, 1, now());
        assert_eq!(p.new_available, 0);
        // With nothing introduced yet, one card.
        let items = drain(&mut Session::new(
            &cards,
            &BTreeMap::new(),
            &prefs,
            0,
            now(),
        ));
        let mut expected = intros("A");
        expected.push(rev("A"));
        assert_eq!(items, expected);
        // Over the cap does not underflow.
        let p = plan(&cards, &BTreeMap::new(), &prefs, 5, now());
        assert_eq!(p.new_available, 0);
    }

    #[test]
    fn again_returns_three_items_later_at_most_twice() {
        let cards: Vec<CardRef> = (1..=8).map(|i| card(&format!("r{i}"), &[])).collect();
        let progress: BTreeMap<_, _> = (1..=8)
            .map(|i| (format!("r{i}"), prog(10 - i, 1.0)))
            .collect();
        let mut s = session(&cards, &progress, &Prefs::default());
        let mut items = Vec::new();
        while let Some(item) = s.next() {
            let grade = match &item {
                Item::Review { card, .. } if card == "r1" => Grade::Again,
                _ => Grade::Good,
            };
            s.report(&item, Some(&graded(grade)));
            items.push(item);
        }
        let expected = vec![
            rev("r1"),
            rev("r2"),
            rev("r3"),
            rev("r4"),
            retry("r1"),
            rev("r5"),
            rev("r6"),
            rev("r7"),
            retry("r1"),
            rev("r8"),
        ];
        assert_eq!(items, expected);
    }

    #[test]
    fn retry_with_few_items_left_comes_at_the_end() {
        let cards = [card("x", &[])];
        let progress = BTreeMap::from([("x".to_string(), prog(1, 1.0))]);
        let mut s = session(&cards, &progress, &Prefs::default());
        let first = s.next().unwrap();
        assert_eq!(first, rev("x"));
        s.report(&first, Some(&graded(Grade::Again)));
        assert_eq!(s.remaining(), 1);
        let second = s.next().unwrap();
        assert_eq!(second, retry("x"));
        s.report(&second, Some(&graded(Grade::Good)));
        assert_eq!(s.next(), None);
    }

    #[test]
    fn scenarios_never_appear() {
        let cards = [scenario("s1"), card("A", &[]), scenario("s2")];
        let progress = BTreeMap::from([("s2".to_string(), prog(5, 30.0))]);
        let items = drain(&mut session(&cards, &progress, &Prefs::default()));
        let mut expected = intros("A");
        expected.push(rev("A"));
        assert_eq!(items, expected);
        assert_eq!(
            plan(&cards, &progress, &Prefs::default(), 0, now()),
            Plan {
                due: 0,
                new_available: 1
            }
        );
    }

    #[test]
    fn plan_matches_the_session() {
        let cards = [
            card("d1", &[]),
            card("later", &[]),
            card("d2", &[]),
            card("n1", &["d1"]),
            card("n2", &["missing"]),
            card("n3", &[]),
            card("n4", &["later"]),
        ];
        let mut progress = BTreeMap::from([
            ("d1".to_string(), prog(2, 1.0)),
            ("d2".to_string(), prog(1, 1.0)),
            ("later".to_string(), prog(-5, 1.0)),
        ]);
        // A card with progress but no reviews does not unlock its dependants.
        progress.get_mut("later").unwrap().reviews = 0;
        let prefs = Prefs::default();
        let p = plan(&cards, &progress, &prefs, 0, now());
        assert_eq!(
            p,
            Plan {
                due: 2,
                new_available: 2
            }
        );
        let items = drain(&mut session(&cards, &progress, &prefs));
        let reviewed: BTreeSet<&str> = items
            .iter()
            .filter_map(|i| match i {
                Item::Review { card, .. } if progress.contains_key(card) => Some(card.as_str()),
                _ => None,
            })
            .collect();
        let introduced: BTreeSet<&str> = items
            .iter()
            .filter_map(|i| match i {
                Item::Intro { card, .. } => Some(card.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(reviewed, BTreeSet::from(["d1", "d2"]));
        assert_eq!(introduced, BTreeSet::from(["n1", "n3"]));
        assert_eq!(reviewed.len(), p.due);
        assert_eq!(introduced.len(), p.new_available);
    }

    #[test]
    fn unknown_ids_in_progress_are_ignored() {
        let cards = [card("A", &[])];
        let progress = BTreeMap::from([
            ("A".to_string(), prog(1, 1.0)),
            ("gone".to_string(), prog(3, 1.0)),
        ]);
        let p = plan(&cards, &progress, &Prefs::default(), 0, now());
        assert_eq!(
            p,
            Plan {
                due: 1,
                new_available: 0
            }
        );
        let items = drain(&mut session(&cards, &progress, &Prefs::default()));
        assert_eq!(items, vec![rev("A")]);
    }

    fn wild_flags(mode: WildMode) -> Vec<(String, bool)> {
        // s1, s2, s3 are stable (>= 14 days); y is young.
        let cards = [
            card("s1", &[]),
            card("y", &[]),
            card("s2", &[]),
            card("s3", &[]),
        ];
        let progress = BTreeMap::from([
            ("s1".to_string(), prog(4, 14.0)),
            ("y".to_string(), prog(3, 13.9)),
            ("s2".to_string(), prog(2, 20.0)),
            ("s3".to_string(), prog(1, 30.0)),
        ]);
        let prefs = Prefs {
            wild_mode: mode,
            ..Prefs::default()
        };
        drain(&mut session(&cards, &progress, &prefs))
            .into_iter()
            .map(|i| match i {
                Item::Review { card, wild, .. } => (card, wild),
                Item::Intro { .. } => panic!("no intros expected"),
            })
            .collect()
    }

    fn flags(v: &[(&str, bool)]) -> Vec<(String, bool)> {
        v.iter().map(|(c, w)| (c.to_string(), *w)).collect()
    }

    #[test]
    fn wild_off_never() {
        assert_eq!(
            wild_flags(WildMode::Off),
            flags(&[("s1", false), ("y", false), ("s2", false), ("s3", false)])
        );
    }

    #[test]
    fn wild_always_for_stable_cards() {
        assert_eq!(
            wild_flags(WildMode::Always),
            flags(&[("s1", true), ("y", false), ("s2", true), ("s3", true)])
        );
    }

    #[test]
    fn wild_sometimes_on_alternate_eligible_reviews() {
        assert_eq!(
            wild_flags(WildMode::Sometimes),
            flags(&[("s1", false), ("y", false), ("s2", true), ("s3", false)])
        );
    }

    #[test]
    fn new_cards_are_never_wild() {
        let cards = [card("A", &[])];
        let prefs = Prefs {
            wild_mode: WildMode::Always,
            ..Prefs::default()
        };
        let items = drain(&mut session(&cards, &BTreeMap::new(), &prefs));
        assert!(items
            .iter()
            .all(|i| !matches!(i, Item::Review { wild: true, .. })));
    }

    #[test]
    fn retries_are_never_wild() {
        let cards = [card("s", &[])];
        let progress = BTreeMap::from([("s".to_string(), prog(1, 30.0))]);
        let prefs = Prefs {
            wild_mode: WildMode::Always,
            ..Prefs::default()
        };
        let mut s = session(&cards, &progress, &prefs);
        let first = s.next().unwrap();
        assert_eq!(
            first,
            Item::Review {
                card: "s".to_string(),
                wild: true,
                retry: false
            }
        );
        s.report(&first, Some(&graded(Grade::Again)));
        assert_eq!(s.next(), Some(retry("s")));
    }

    #[test]
    fn intros_are_never_interrupted() {
        let cards = [card("A", &[]), card("B", &[]), card("C", &[])];
        let mut s = session(&cards, &BTreeMap::new(), &Prefs::default());
        assert_eq!(s.remaining(), 12);
        let mut expected = intros("A");
        expected.extend(intros("B"));
        expected.push(rev("A"));
        expected.extend(intros("C"));
        expected.extend([rev("B"), rev("C")]);
        let mut items = Vec::new();
        while let Some(item) = s.next() {
            items.push(item);
            // remaining() counts down by one per served item.
            assert_eq!(s.remaining(), 12 - items.len());
        }
        assert_eq!(items, expected);
    }

    #[test]
    fn empty_and_all_scenario_packs_have_nothing() {
        let none = Plan {
            due: 0,
            new_available: 0,
        };
        let prefs = Prefs::default();
        assert_eq!(session(&[], &BTreeMap::new(), &prefs).next(), None);
        assert_eq!(plan(&[], &BTreeMap::new(), &prefs, 0, now()), none);
        let cards = [scenario("s1"), scenario("s2")];
        let mut s = session(&cards, &BTreeMap::new(), &prefs);
        assert_eq!(s.remaining(), 0);
        assert_eq!(s.next(), None);
        assert_eq!(plan(&cards, &BTreeMap::new(), &prefs, 0, now()), none);
    }

    fn record(at: DateTime<Utc>, pack: &str, card: &str, intro: bool) -> ReviewRecord {
        ReviewRecord {
            at,
            pack: pack.to_string(),
            card: card.to_string(),
            grade: Grade::Good,
            time_ms: 1000,
            target_ms: 3000,
            binding_keys: 2,
            typed_chars: 0,
            canonical_keys: 2,
            canonical_chars: 0,
            fail: None,
            capped: false,
            wild: false,
            intro,
        }
    }

    #[test]
    fn introduced_today_uses_the_local_date() {
        let cst = FixedOffset::west_opt(6 * 3600).unwrap();
        // Local 2026-11-26 runs from 06:00Z on the 26th to 06:00Z on the 27th.
        let now = Utc.with_ymd_and_hms(2026, 11, 27, 5, 0, 0).unwrap(); // 23:00 local
        let at = |d, h, m| Utc.with_ymd_and_hms(2026, 11, d, h, m, 0).unwrap();
        let reviews = vec![
            // Local 2026-11-25 23:59: yesterday.
            record(at(26, 5, 59), "tmux", "yesterday", true),
            // Local 2026-11-26 00:00: today.
            record(at(26, 6, 0), "tmux", "a", true),
            record(at(26, 7, 0), "tmux", "a", false),
            // 04:00Z on the 27th is 22:00 local on the 26th: today.
            record(at(27, 4, 0), "tmux", "b", false),
            // First seen yesterday, reviewed again today: not new today.
            record(at(26, 8, 0), "tmux", "yesterday", false),
            // Another pack.
            record(at(26, 9, 0), "vim", "c", true),
        ];
        assert_eq!(introduced_today(&reviews, "tmux", now, cst), 2);
        assert_eq!(introduced_today(&reviews, "vim", now, cst), 1);
        assert_eq!(introduced_today(&[], "tmux", now, cst), 0);
        // In UTC, 05:59Z on the 26th is the 26th, but `now` is the 27th.
        let utc = FixedOffset::east_opt(0).unwrap();
        assert_eq!(introduced_today(&reviews, "tmux", now, utc), 1);
    }

    #[test]
    fn introduced_today_uses_the_earliest_record_in_any_order() {
        let cst = FixedOffset::west_opt(6 * 3600).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 26, 18, 0, 0).unwrap();
        let reviews = vec![
            record(now - Duration::hours(1), "tmux", "a", false),
            record(now - Duration::days(3), "tmux", "a", true),
        ];
        assert_eq!(introduced_today(&reviews, "tmux", now, cst), 0);
    }
}
