//! Session statistics: the combo counter, the end-of-session summary, and
//! the calendar figures the home screen shows. Pure functions; callers pass
//! `now` and the local UTC offset.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Datelike, Days, FixedOffset, NaiveDate, TimeZone, Utc};

use crate::grade::{Grade, Graded};
use crate::schedule::CardProgress;
use crate::session::Plan;
use crate::store::ReviewRecord;

/// A run of consecutive non-Again results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Combo {
    current: u32,
    best: u32,
}

impl Combo {
    /// Any grade but Again extends the run; Again resets it silently.
    pub fn push(&mut self, g: Grade) {
        if g == Grade::Again {
            self.current = 0;
        } else {
            self.current = self.current.saturating_add(1);
            self.best = self.best.max(self.current);
        }
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    pub fn best(&self) -> u32 {
        self.best
    }
}

/// The end-of-session summary (DESIGN §7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Reviews done, not counting retries.
    pub cards: u32,
    pub time: Duration,
    pub best_combo: u32,
    /// Results per grade: Again, Hard, Good, Easy (retries included).
    pub per_grade: [u32; 4],
    /// Personal bests set this session, in the order they happened.
    pub new_bests: Vec<(String, Duration)>,
    pub next_reviews_tomorrow: u32,
    pub new_waiting: u32,
}

/// Collects results as a session runs.
#[derive(Clone, Debug, Default)]
pub struct SummaryBuilder {
    cards: u32,
    combo: Combo,
    per_grade: [u32; 4],
    new_bests: Vec<(String, Duration)>,
}

fn grade_index(g: Grade) -> usize {
    match g {
        Grade::Again => 0,
        Grade::Hard => 1,
        Grade::Good => 2,
        Grade::Easy => 3,
    }
}

impl SummaryBuilder {
    pub fn new() -> SummaryBuilder {
        SummaryBuilder::default()
    }

    /// Record one graded attempt. Retries count in the grades and the combo
    /// but not in `cards`.
    pub fn add(&mut self, card: &str, graded: &Graded, new_best: bool, retry: bool) {
        if !retry {
            self.cards = self.cards.saturating_add(1);
        }
        self.combo.push(graded.grade);
        if let Some(n) = self.per_grade.get_mut(grade_index(graded.grade)) {
            *n = n.saturating_add(1);
        }
        if new_best {
            self.new_bests.push((card.to_string(), graded.time));
        }
    }

    pub fn combo(&self) -> &Combo {
        &self.combo
    }

    pub fn finish(self, session_time: Duration, plan_tomorrow: &Plan) -> Summary {
        Summary {
            cards: self.cards,
            time: session_time,
            best_combo: self.combo.best(),
            per_grade: self.per_grade,
            new_bests: self.new_bests,
            next_reviews_tomorrow: u32::try_from(plan_tomorrow.due).unwrap_or(u32::MAX),
            new_waiting: u32::try_from(plan_tomorrow.new_available).unwrap_or(u32::MAX),
        }
    }
}

fn local_date(at: DateTime<Utc>, offset: FixedOffset) -> NaiveDate {
    at.with_timezone(&offset).date_naive()
}

/// Distinct local dates with any review activity in the current local month.
pub fn days_this_month(reviews: &[ReviewRecord], now: DateTime<Utc>, offset: FixedOffset) -> u32 {
    let today = local_date(now, offset);
    let dates: BTreeSet<NaiveDate> = reviews
        .iter()
        .map(|r| local_date(r.at, offset))
        .filter(|d| d.year() == today.year() && d.month() == today.month())
        .collect();
    u32::try_from(dates.len()).unwrap_or(u32::MAX)
}

/// Cards due before the end of the next local day (including cards already due).
pub fn due_tomorrow(
    progress: &BTreeMap<String, CardProgress>,
    now: DateTime<Utc>,
    offset: FixedOffset,
) -> u32 {
    let end = local_date(now, offset)
        .checked_add_days(Days::new(2))
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|t| offset.from_local_datetime(&t).single())
        .map(|t| t.with_timezone(&Utc));
    let Some(end) = end else { return 0 };
    let n = progress.values().filter(|p| p.due() < end).count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::schedule::new_card;

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

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn rec(at: &str) -> ReviewRecord {
        ReviewRecord {
            at: utc(at),
            pack: "tmux".into(),
            card: "c".into(),
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
            intro: false,
        }
    }

    fn prog(due: &str) -> CardProgress {
        let mut fsrs = new_card(utc("2026-01-01T00:00:00Z"));
        fsrs.due = utc(due);
        CardProgress {
            fsrs,
            reviews: 1,
            best: None,
            last_times: Vec::new(),
        }
    }

    #[test]
    fn combo_resets_on_again() {
        let mut c = Combo::default();
        for g in [
            Grade::Good,
            Grade::Easy,
            Grade::Hard,
            Grade::Again,
            Grade::Good,
        ] {
            c.push(g);
        }
        assert_eq!((c.current(), c.best()), (1, 3));
    }

    #[test]
    fn summary_counts_grades_and_skips_retries_in_cards() {
        let mut b = SummaryBuilder::new();
        b.add("a", &graded(Grade::Again, 5000), false, false);
        b.add("a", &graded(Grade::Good, 2000), true, true);
        b.add("b", &graded(Grade::Easy, 900), true, false);
        b.add("c", &graded(Grade::Hard, 4000), false, false);
        let plan = Plan {
            due: 7,
            new_available: 4,
        };
        let s = b.finish(Duration::from_secs(60), &plan);
        assert_eq!(s.cards, 3);
        assert_eq!(s.per_grade, [1, 1, 1, 1]);
        assert_eq!(s.best_combo, 3);
        assert_eq!(s.time, Duration::from_secs(60));
        assert_eq!(
            s.new_bests,
            vec![
                ("a".to_string(), Duration::from_millis(2000)),
                ("b".to_string(), Duration::from_millis(900)),
            ]
        );
        assert_eq!((s.next_reviews_tomorrow, s.new_waiting), (7, 4));
    }

    #[test]
    fn days_this_month_counts_distinct_local_dates() {
        let off = FixedOffset::west_opt(5 * 3600).unwrap();
        let now = utc("2026-11-26T15:00:00Z");
        let reviews = [
            rec("2026-11-03T15:00:00Z"),
            rec("2026-11-10T15:00:00Z"),
            rec("2026-11-10T16:00:00Z"),
            // 10:00 local on the 20th (the 20th in UTC too).
            rec("2026-11-20T15:00:00Z"),
            // 23:30 local on the 20th is the 21st in UTC: the same local day
            // as the record above, so the two count once.
            rec("2026-11-21T04:30:00Z"),
            // last month
            rec("2026-10-31T15:00:00Z"),
        ];
        assert_eq!(days_this_month(&reviews, now, off), 3);
        // 01:00 UTC on Nov 1 is still Oct 31 locally: not this month.
        assert_eq!(days_this_month(&[rec("2026-11-01T01:00:00Z")], now, off), 0);
    }

    #[test]
    fn due_tomorrow_ends_at_local_midnight() {
        let off = FixedOffset::west_opt(5 * 3600).unwrap();
        // 10:00 local on the 26th.
        let now = utc("2026-11-26T15:00:00Z");
        // End of tomorrow = 2026-11-28T00:00 local = 05:00 UTC.
        let progress = BTreeMap::from([
            ("past".to_string(), prog("2026-11-20T00:00:00Z")),
            ("tomorrow".to_string(), prog("2026-11-28T04:59:59Z")),
            ("edge".to_string(), prog("2026-11-28T05:00:00Z")),
            ("later".to_string(), prog("2026-12-30T00:00:00Z")),
        ]);
        assert_eq!(due_tomorrow(&progress, now, off), 2);
    }
}
