//! FSRS scheduling and mastery bands (spec §6.5).
//!
//! The engine never reads the clock: callers pass `now`.

use std::time::Duration;

use chrono::{DateTime, Utc};
use rs_fsrs::{Card, Parameters, Rating, Seed, State, FSRS};
use serde::{Deserialize, Serialize};

use crate::grade::{Grade, Graded};

/// Successful times kept per card.
const MAX_TIMES: usize = 10;
/// Stability (days) at which a card counts as mature.
const MATURE_STABILITY: f64 = 21.0;

// Durations serialize as integer milliseconds (controller ruling 3).
mod millis_opt {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Option<Duration>, s: S) -> Result<S::Ok, S::Error> {
        match d {
            Some(d) => s.serialize_some(&u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
        Ok(Option::<u64>::deserialize(d)?.map(Duration::from_millis))
    }
}

mod millis_vec {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[Duration], s: S) -> Result<S::Ok, S::Error> {
        let ms: Vec<u64> = v
            .iter()
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .collect();
        ms.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Duration>, D::Error> {
        Ok(Vec::<u64>::deserialize(d)?
            .into_iter()
            .map(Duration::from_millis)
            .collect())
    }
}

/// How well learned a card is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mastery {
    New,
    Learning,
    Young,
    Mature,
}

/// Everything remembered about one card.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CardProgress {
    pub fsrs: Card,
    pub reviews: u32,
    #[serde(with = "millis_opt")]
    pub best: Option<Duration>,
    /// Successful times, newest last, at most 10.
    #[serde(with = "millis_vec")]
    pub last_times: Vec<Duration>,
}

impl CardProgress {
    pub fn mastery(&self) -> Mastery {
        match self.fsrs.state {
            State::New => Mastery::New,
            State::Learning | State::Relearning => Mastery::Learning,
            State::Review if self.fsrs.stability >= MATURE_STABILITY => Mastery::Mature,
            State::Review => Mastery::Young,
        }
    }

    pub fn due(&self) -> DateTime<Utc> {
        self.fsrs.due
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.fsrs.due <= now
    }
}

/// A never-reviewed FSRS card, due at `now`.
pub(crate) fn new_card(now: DateTime<Utc>) -> Card {
    Card {
        due: now,
        last_review: now,
        stability: 0.0,
        difficulty: 0.0,
        elapsed_days: 0,
        scheduled_days: 0,
        reps: 0,
        lapses: 0,
        state: State::New,
    }
}

/// Schedules reviews with FSRS (retention 0.9, short-term on, fuzz off).
pub struct Scheduler {
    fsrs: FSRS,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            fsrs: FSRS::new(Parameters {
                request_retention: 0.9,
                enable_short_term: true,
                enable_fuzz: false,
                // A fixed seed: the default one is the current time.
                seed: Seed::new("keyfluent"),
                ..Default::default()
            }),
        }
    }

    /// Apply one graded attempt. Returns the new progress and whether it set a
    /// new best time.
    pub fn review(
        &self,
        progress: Option<&CardProgress>,
        graded: &Graded,
        now: DateTime<Utc>,
    ) -> (CardProgress, bool) {
        let mut next = match progress {
            Some(p) => p.clone(),
            None => CardProgress {
                fsrs: new_card(now),
                reviews: 0,
                best: None,
                last_times: Vec::new(),
            },
        };
        // A clock that went backwards must not reach rs-fsrs.
        let now = now.max(next.fsrs.last_review);
        let rating = match graded.grade {
            Grade::Again => Rating::Again,
            Grade::Hard => Rating::Hard,
            Grade::Good => Rating::Good,
            Grade::Easy => Rating::Easy,
        };
        let scheduled = self.fsrs.next(next.fsrs.clone(), now, rating).card;
        // rs-fsrs can compute NaN from odd stored values (stability 0 in
        // Review is 0 * inf); such a card restarts rather than poisoning the
        // saved progress.
        next.fsrs = if scheduled.stability.is_finite() && scheduled.difficulty.is_finite() {
            scheduled
        } else {
            new_card(now)
        };
        next.reviews = next.reviews.saturating_add(1);

        let mut new_best = false;
        if graded.grade != Grade::Again {
            new_best = next.best.is_none_or(|b| graded.time < b);
            if new_best {
                next.best = Some(graded.time);
            }
            next.last_times.push(graded.time);
            if next.last_times.len() > MAX_TIMES {
                let excess = next.last_times.len() - MAX_TIMES;
                next.last_times.drain(..excess);
            }
        }
        (next, new_best)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn graded(grade: Grade, ms: u64) -> Graded {
        Graded {
            grade,
            time: Duration::from_millis(ms),
            target: Duration::from_secs(3),
            binding_keys: 2,
            typed_chars: 0,
            canonical_keys: 2,
            canonical_chars: 0,
            fail: None,
            capped: false,
            practiced: Vec::new(),
        }
    }

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
    }

    #[test]
    fn first_good_is_learning_and_new_best() {
        let s = Scheduler::new();
        let (p, best) = s.review(None, &graded(Grade::Good, 2000), t0());
        assert_eq!(p.mastery(), Mastery::Learning);
        assert!(p.due() > t0());
        assert!(!p.is_due(t0()));
        assert_eq!(p.reviews, 1);
        assert_eq!(p.best, Some(Duration::from_millis(2000)));
        assert!(best);
    }

    #[test]
    fn slower_good_is_not_new_best_and_faster_is() {
        let s = Scheduler::new();
        let (p, _) = s.review(None, &graded(Grade::Good, 2000), t0());
        let (p, best) = s.review(Some(&p), &graded(Grade::Good, 2500), t0());
        assert!(!best);
        assert_eq!(p.best, Some(Duration::from_millis(2000)));
        let (p, best) = s.review(Some(&p), &graded(Grade::Good, 1500), t0());
        assert!(best);
        assert_eq!(p.best, Some(Duration::from_millis(1500)));
    }

    #[test]
    fn again_leaves_best_and_times() {
        let s = Scheduler::new();
        let (p, _) = s.review(None, &graded(Grade::Good, 2000), t0());
        let (p, best) = s.review(Some(&p), &graded(Grade::Again, 100), t0());
        assert!(!best);
        assert_eq!(p.best, Some(Duration::from_millis(2000)));
        assert_eq!(p.last_times.len(), 1);
        assert_eq!(p.reviews, 2);
        let (p, best) = s.review(None, &graded(Grade::Again, 100), t0());
        assert!(!best);
        assert_eq!(p.best, None);
    }

    #[test]
    fn last_times_capped_at_ten_newest_last() {
        let s = Scheduler::new();
        let mut p = None;
        for i in 0..14u64 {
            p = Some(s.review(p.as_ref(), &graded(Grade::Good, 1000 + i), t0()).0);
        }
        let p = p.unwrap();
        assert_eq!(p.last_times.len(), 10);
        assert_eq!(p.last_times[9], Duration::from_millis(1013));
        assert_eq!(p.last_times[0], Duration::from_millis(1004));
    }

    #[test]
    fn repeated_easy_reaches_mature() {
        let s = Scheduler::new();
        let mut now = t0();
        let mut p: Option<CardProgress> = None;
        for _ in 0..8 {
            let (n, _) = s.review(p.as_ref(), &graded(Grade::Easy, 1000), now);
            now = n.due();
            p = Some(n);
        }
        assert_eq!(p.unwrap().mastery(), Mastery::Mature);
    }

    #[test]
    fn now_before_last_review_is_clamped() {
        let s = Scheduler::new();
        let (p, _) = s.review(None, &graded(Grade::Good, 2000), t0());
        let earlier = t0() - chrono::Duration::days(3);
        let (p2, _) = s.review(Some(&p), &graded(Grade::Good, 2000), earlier);
        assert!(p2.fsrs.last_review >= p.fsrs.last_review);
        assert!(p2.due() >= p.fsrs.last_review);
    }

    #[test]
    fn serde_round_trip_uses_milliseconds() {
        let s = Scheduler::new();
        let (p, _) = s.review(None, &graded(Grade::Good, 2345), t0());
        let text = toml::to_string(&p).unwrap();
        assert!(text.contains("2345"));
        let back: CardProgress = toml::from_str(&text).unwrap();
        assert_eq!(back, p);
    }

    fn review_state(stability: f64, difficulty: f64) -> CardProgress {
        let mut fsrs = new_card(t0());
        fsrs.state = State::Review;
        fsrs.stability = stability;
        fsrs.difficulty = difficulty;
        fsrs.reps = 3;
        fsrs.scheduled_days = 2;
        CardProgress {
            fsrs,
            reviews: 3,
            best: Some(Duration::from_millis(1500)),
            last_times: vec![Duration::from_millis(1500)],
        }
    }

    #[test]
    fn non_finite_results_restart_the_card() {
        let s = Scheduler::new();
        let later = t0() + chrono::Duration::days(2);
        for (card, grade) in [
            (review_state(0.0, 5.0), Grade::Good),
            (review_state(5.0, 0.0), Grade::Again),
        ] {
            let (p, _) = s.review(Some(&card), &graded(grade, 2000), later);
            assert!(p.fsrs.stability.is_finite(), "{p:?}");
            assert!(p.fsrs.difficulty.is_finite(), "{p:?}");
            assert_eq!(p.reviews, 4);
            assert_eq!(p.best, Some(Duration::from_millis(1500)));
        }
    }
}
