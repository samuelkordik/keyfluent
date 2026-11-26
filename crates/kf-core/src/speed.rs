//! The learner's personal reaction time (spec §6.4).

use std::time::Duration;

use crate::grade::{Grade, Graded, Speed, Target};

/// Successful reviews needed before the reaction time is personalised, and
/// how many of the most recent ones are used.
const WINDOW: usize = 50;
const MIN_REACTION: f64 = 0.8;
const MAX_REACTION: f64 = 3.0;

/// One past attempt, reduced to what the reaction estimate needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub time: Duration,
    /// Canonical binding keys of the card.
    pub binding_keys: usize,
    /// Canonical typed characters of the card.
    pub typed_chars: usize,
    pub success: bool,
}

impl From<&Graded> for Sample {
    fn from(g: &Graded) -> Sample {
        Sample {
            time: g.time,
            binding_keys: g.canonical_keys,
            typed_chars: g.canonical_chars,
            success: g.grade != Grade::Again,
        }
    }
}

/// The time model personalised from history: with fewer than 50 successful
/// samples it is the standard one; otherwise the reaction term is the median
/// of `time - per_key * keys - per_char * chars` over the last 50 successful
/// samples, clamped to 0.8–3.0 s.
pub fn personal_target(speed: Speed, history: &[Sample]) -> Target {
    let standard = Target::standard(speed);
    let mut residuals: Vec<f64> = history
        .iter()
        .rev()
        .filter(|s| s.success)
        .take(WINDOW)
        .map(|s| {
            let keys = standard.per_key.as_secs_f64() * s.binding_keys as f64;
            let chars = standard.per_char.as_secs_f64() * s.typed_chars as f64;
            s.time.as_secs_f64() - keys - chars
        })
        .collect();
    if residuals.len() < WINDOW {
        return standard;
    }
    residuals.sort_by(f64::total_cmp);
    let mid = residuals.len() / 2;
    let median = match (residuals.get(mid.wrapping_sub(1)), residuals.get(mid)) {
        (Some(a), Some(b)) => (a + b) / 2.0,
        _ => return standard,
    };
    if !median.is_finite() {
        return standard;
    }
    Target {
        reaction: Duration::from_secs_f64(median.clamp(MIN_REACTION, MAX_REACTION)),
        ..standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ms: u64, keys: usize, success: bool) -> Sample {
        Sample {
            time: Duration::from_millis(ms),
            binding_keys: keys,
            typed_chars: 0,
            success,
        }
    }

    fn reaction(history: &[Sample]) -> Duration {
        personal_target(Speed::Normal, history).reaction
    }

    #[test]
    fn fewer_than_fifty_is_standard() {
        let h = vec![sample(2300, 2, true); 49];
        assert_eq!(
            personal_target(Speed::Brisk, &h),
            Target::standard(Speed::Brisk)
        );
    }

    #[test]
    fn fifty_reviews_set_the_reaction() {
        let h = vec![sample(2300, 2, true); 50];
        let t = personal_target(Speed::Relaxed, &h);
        assert_eq!(t.reaction, Duration::from_millis(1500));
        let std = Target::standard(Speed::Relaxed);
        assert_eq!(
            (t.per_key, t.per_char, t.factor),
            (std.per_key, std.per_char, std.factor)
        );
    }

    #[test]
    fn reaction_is_clamped() {
        assert_eq!(
            reaction(&vec![sample(900, 2, true); 50]),
            Duration::from_millis(800)
        );
        assert_eq!(
            reaction(&vec![sample(9000, 2, true); 50]),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn again_reviews_are_ignored() {
        let mut h = vec![sample(2300, 2, true); 49];
        h.extend(vec![sample(9000, 2, false); 10]);
        assert_eq!(reaction(&h), Target::standard(Speed::Normal).reaction);
        h.push(sample(2300, 2, true));
        assert_eq!(reaction(&h), Duration::from_millis(1500));
    }

    #[test]
    fn only_the_last_fifty_count() {
        let mut h = vec![sample(2800, 2, true); 20];
        h.extend(vec![sample(2300, 2, true); 50]);
        assert_eq!(reaction(&h), Duration::from_millis(1500));
    }

    #[test]
    fn typed_chars_count_and_the_median_is_not_the_mean() {
        // 30 cards of 1 key + 4 chars with a 1.0 s reaction (1.0 + 0.4 +
        // 0.6 s) and 20 cards of 2 keys with a 3.0 s reaction (3.0 + 0.8 s):
        // the median reaction is 1.0 s, the mean would be 1.8 s.
        let typed = Sample {
            time: Duration::from_millis(2000),
            binding_keys: 1,
            typed_chars: 4,
            success: true,
        };
        let mut h = vec![typed; 30];
        h.extend(vec![sample(3800, 2, true); 20]);
        let r = reaction(&h);
        let diff = r.abs_diff(Duration::from_millis(1000));
        assert!(diff < Duration::from_millis(1), "{r:?}");
    }

    #[test]
    fn graded_converts_with_canonical_counts() {
        use crate::grade::{Attempt, CardShape};
        let shape = CardShape {
            binding_keys: 2,
            typed_chars: 1,
            practiced: vec![],
        };
        let g = Attempt::new(shape, Duration::from_secs(2)).succeed(Duration::from_secs(1));
        let s = Sample::from(&g);
        assert_eq!((s.binding_keys, s.typed_chars, s.success), (2, 1, true));
    }
}
