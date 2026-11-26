//! Grading one attempt at a card (spec §6.4).
//!
//! The engine never reads the clock: callers pass attempt-relative
//! [`Duration`]s.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{KeyOutcome, RanBinding};

/// Binding keys beyond the canonical count that are tolerated; one more fails.
const SLACK: usize = 3;
/// Binding keys beyond the canonical count that make a correct route Hard.
const HARD_EXTRA: usize = 2;
/// An attempt that runs this long fails.
const TIMEOUT: Duration = Duration::from_secs(30);

/// How well an attempt went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    /// Filled stars out of three: Easy 3, Good 2, Hard 1, Again 0.
    pub fn stars(self) -> u8 {
        match self {
            Grade::Again => 0,
            Grade::Hard => 1,
            Grade::Good => 2,
            Grade::Easy => 3,
        }
    }

    /// The label shown to the learner.
    pub fn label(self) -> &'static str {
        match self {
            Grade::Again => "AGAIN",
            Grade::Hard => "HARD",
            Grade::Good => "GOOD",
            Grade::Easy => "EASY",
        }
    }
}

/// How fast the learner wants to be.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    Relaxed,
    #[default]
    Normal,
    Brisk,
}

impl Speed {
    /// The multiplier on the target time.
    pub fn factor(self) -> f64 {
        match self {
            Speed::Relaxed => 1.5,
            Speed::Normal => 1.0,
            Speed::Brisk => 0.75,
        }
    }
}

/// The time model: `(reaction + per_key * keys + per_char * chars) * factor`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub reaction: Duration,
    pub per_key: Duration,
    pub per_char: Duration,
    pub factor: f64,
}

impl Target {
    /// The standard model: 1.2 s reaction, 0.4 s per binding key, 0.15 s per
    /// typed character, scaled by the speed expectation.
    pub fn standard(speed: Speed) -> Target {
        Target {
            reaction: Duration::from_millis(1200),
            per_key: Duration::from_millis(400),
            per_char: Duration::from_millis(150),
            factor: speed.factor(),
        }
    }

    /// The target time for a card with this many binding keys and typed chars.
    pub fn for_card(&self, binding_keys: usize, typed_chars: usize) -> Duration {
        let raw = self.reaction
            + self
                .per_key
                .saturating_mul(u32::try_from(binding_keys).unwrap_or(u32::MAX))
            + self
                .per_char
                .saturating_mul(u32::try_from(typed_chars).unwrap_or(u32::MAX));
        let secs = raw.as_secs_f64() * self.factor;
        if secs.is_finite() && secs >= 0.0 {
            Duration::try_from_secs_f64(secs).unwrap_or(Duration::MAX)
        } else {
            Duration::ZERO
        }
    }
}

/// What grading needs to know about a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardShape {
    /// Binding keys in the canonical route.
    pub binding_keys: usize,
    /// Typed characters in the canonical route.
    pub typed_chars: usize,
    /// Bindings the canonical route runs (the keys the card practices).
    pub practiced: Vec<RanBinding>,
}

/// Why an attempt failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fail {
    TooManyKeys,
    Timeout,
    Hint,
}

/// An attempt in progress.
#[derive(Clone, Debug)]
pub struct Attempt {
    shape: CardShape,
    target: Duration,
    keys: usize,
    chars: usize,
    ran: Vec<RanBinding>,
}

/// The result of an attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graded {
    pub grade: Grade,
    pub time: Duration,
    pub target: Duration,
    /// Binding keys the learner actually pressed.
    pub binding_keys: usize,
    /// Characters the learner actually typed (text entry).
    pub typed_chars: usize,
    /// The card's canonical binding-key count (what `target` was built from).
    pub canonical_keys: usize,
    /// The card's canonical typed-character count.
    pub canonical_chars: usize,
    pub fail: Option<Fail>,
    /// A fast, correct attempt that skipped a practiced binding was held to Good.
    pub capped: bool,
    /// The bindings the card practices, so the result can name them.
    pub practiced: Vec<RanBinding>,
}

impl Attempt {
    pub fn new(shape: CardShape, target: Duration) -> Attempt {
        Attempt {
            shape,
            target,
            keys: 0,
            chars: 0,
            ran: Vec::new(),
        }
    }

    /// Records one key's outcome. Returns `Some(TooManyKeys)` once binding
    /// keys exceed the canonical count plus the slack. Text entry is free.
    pub fn record(&mut self, outcome: &KeyOutcome, _at: Duration) -> Option<Fail> {
        if outcome.text_entry {
            self.chars = self.chars.saturating_add(1);
            return None;
        }
        self.keys = self.keys.saturating_add(1);
        if let Some(ran) = &outcome.ran {
            self.ran.push(ran.clone());
        }
        (self.keys > self.shape.binding_keys.saturating_add(SLACK)).then_some(Fail::TooManyKeys)
    }

    /// `Some(Timeout)` once the attempt has run 30 s.
    pub fn tick(&mut self, at: Duration) -> Option<Fail> {
        (at >= TIMEOUT).then_some(Fail::Timeout)
    }

    /// The learner asked for the hint: the attempt fails.
    pub fn hint(&mut self) -> Fail {
        Fail::Hint
    }

    /// Grades a successful attempt that finished at `at`.
    pub fn succeed(self, at: Duration) -> Graded {
        let canonical = self.shape.binding_keys;
        let (grade, capped) = if at > self.target.saturating_mul(2)
            || self.keys > canonical.saturating_add(HARD_EXTRA)
        {
            (Grade::Hard, false)
        } else if at < self.target && self.keys <= canonical {
            if self.covers_practiced() {
                (Grade::Easy, false)
            } else {
                (Grade::Good, true)
            }
        } else {
            (Grade::Good, false)
        };
        self.finish(grade, at, None, capped)
    }

    /// Grades a failed attempt: always Again.
    pub fn fail(self, why: Fail, at: Duration) -> Graded {
        self.finish(Grade::Again, at, Some(why), false)
    }

    fn covers_practiced(&self) -> bool {
        self.shape.practiced.iter().all(|p| {
            self.ran
                .iter()
                .any(|r| r.table == p.table && r.key == p.key)
        })
    }

    fn finish(self, grade: Grade, time: Duration, fail: Option<Fail>, capped: bool) -> Graded {
        Graded {
            grade,
            time,
            target: self.target,
            binding_keys: self.keys,
            typed_chars: self.chars,
            canonical_keys: self.shape.binding_keys,
            canonical_chars: self.shape.typed_chars,
            fail,
            capped,
            practiced: self.shape.practiced,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::key::Key;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn binding(k: &str) -> RanBinding {
        RanBinding {
            table: "prefix".into(),
            key: k.parse::<Key>().unwrap(),
            command: "x".into(),
        }
    }

    fn press(ran: Option<RanBinding>) -> KeyOutcome {
        KeyOutcome {
            text_entry: false,
            ran,
        }
    }

    fn shape(keys: usize, practiced: Vec<RanBinding>) -> CardShape {
        CardShape {
            binding_keys: keys,
            typed_chars: 0,
            practiced,
        }
    }

    fn attempt(canonical: usize, presses: usize) -> Attempt {
        let mut a = Attempt::new(shape(canonical, vec![]), ms(2000));
        for _ in 0..presses {
            assert_eq!(a.record(&press(None), ms(0)), None);
        }
        a
    }

    #[test]
    fn target_scales_with_keys_chars_and_speed() {
        assert_eq!(Target::standard(Speed::Normal).for_card(2, 0), ms(2000));
        assert_eq!(Target::standard(Speed::Relaxed).for_card(2, 0), ms(3000));
        assert_eq!(Target::standard(Speed::Brisk).for_card(2, 0), ms(1500));
        assert_eq!(Target::standard(Speed::Normal).for_card(2, 4), ms(2600));
    }

    #[test]
    fn stars_and_labels() {
        let all = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy];
        assert_eq!(all.map(Grade::stars), [0, 1, 2, 3]);
        assert_eq!(all.map(Grade::label), ["AGAIN", "HARD", "GOOD", "EASY"]);
    }

    #[test]
    fn speed_and_grade_serde_lowercase() {
        assert_eq!(serde_json_like(Speed::Brisk), "\"brisk\"");
        assert_eq!(serde_json_like(Grade::Easy), "\"easy\"");
        assert_eq!(serde_json_like(Fail::TooManyKeys), "\"too_many_keys\"");
        assert_eq!(serde_json_like(Fail::Hint), "\"hint\"");
    }

    fn serde_json_like<T: Serialize>(v: T) -> String {
        #[derive(Serialize)]
        struct W<T> {
            v: T,
        }
        let s = toml::to_string(&W { v }).unwrap();
        s.trim()
            .strip_prefix("v = ")
            .unwrap()
            .to_string()
            .replace('\'', "\"")
    }

    #[test]
    fn grades_by_time() {
        let g = attempt(2, 2).succeed(ms(1500));
        assert_eq!(
            (g.grade, g.grade.stars(), g.capped),
            (Grade::Easy, 3, false)
        );
        assert_eq!(attempt(2, 2).succeed(ms(2500)).grade, Grade::Good);
        assert_eq!(attempt(2, 2).succeed(ms(4100)).grade, Grade::Hard);
    }

    #[test]
    fn extra_keys_downgrade() {
        assert_eq!(attempt(2, 5).succeed(ms(1000)).grade, Grade::Hard);
        assert_eq!(attempt(2, 3).succeed(ms(1000)).grade, Grade::Good);
    }

    #[test]
    fn graded_carries_pressed_and_canonical_counts() {
        let mut a = Attempt::new(shape(2, vec![]), ms(2000));
        for _ in 0..4 {
            a.record(&press(None), ms(0));
        }
        let typed = KeyOutcome {
            text_entry: true,
            ran: None,
        };
        for _ in 0..3 {
            a.record(&typed, ms(0));
        }
        let g = a.succeed(ms(1000));
        assert_eq!(
            (
                g.binding_keys,
                g.typed_chars,
                g.canonical_keys,
                g.canonical_chars
            ),
            (4, 3, 2, 0)
        );
    }

    #[test]
    fn too_many_keys_fails() {
        let mut a = attempt(2, 5);
        assert_eq!(a.record(&press(None), ms(0)), Some(Fail::TooManyKeys));
    }

    #[test]
    fn text_entry_never_counts() {
        let mut a = Attempt::new(shape(1, vec![]), ms(2000));
        let typed = KeyOutcome {
            text_entry: true,
            ran: None,
        };
        for _ in 0..50 {
            assert_eq!(a.record(&typed, ms(0)), None);
        }
        assert_eq!(a.succeed(ms(500)).grade, Grade::Easy);
    }

    #[test]
    fn timeout_and_hint() {
        let mut a = attempt(2, 0);
        assert_eq!(a.tick(ms(29_999)), None);
        assert_eq!(a.tick(Duration::from_secs(30)), Some(Fail::Timeout));
        let why = a.hint();
        assert_eq!(why, Fail::Hint);
        let g = a.fail(why, ms(900));
        assert_eq!(
            (g.grade, g.grade.stars(), g.fail),
            (Grade::Again, 0, Some(Fail::Hint))
        );
    }

    #[test]
    fn missing_practiced_binding_caps_at_good() {
        let want = binding("%");
        let mut a = Attempt::new(shape(2, vec![want.clone()]), ms(2000));
        a.record(&press(None), ms(0));
        a.record(&press(Some(binding("\""))), ms(0));
        let g = a.succeed(ms(1000));
        assert_eq!((g.grade, g.capped), (Grade::Good, true));
        assert_eq!(g.practiced, vec![want.clone()]);

        let mut a = Attempt::new(shape(2, vec![want.clone()]), ms(2000));
        a.record(&press(None), ms(0));
        a.record(&press(Some(want)), ms(0));
        let g = a.succeed(ms(1000));
        assert_eq!((g.grade, g.capped), (Grade::Easy, false));
    }
}
