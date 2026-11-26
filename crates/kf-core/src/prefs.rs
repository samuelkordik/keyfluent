//! Learner preferences (spec §6.8).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::grade::Speed;

/// Wild mode: the goal picture is hidden and only the intent sentence shows
/// (applied only to cards with stability >= 14 days).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WildMode {
    Off,
    #[default]
    Sometimes,
    Always,
}

/// The key set used in tmux copy mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyModeKeys {
    #[default]
    Vi,
    Emacs,
}

/// Everything the learner can set. Every field has a default, so files
/// written by older or newer versions load: missing fields take the default
/// and unknown fields are ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// New cards introduced per day; `None` means no limit.
    pub daily_new_cards: Option<u32>,
    pub wild_mode: WildMode,
    pub speed: Speed,
    /// Days of practice before scenarios unlock.
    pub scenario_unlock_days: u32,
    pub copy_mode_keys: CopyModeKeys,
    /// Treat the macOS Option key as Meta.
    pub option_as_meta: bool,
    /// The chord that pauses a session.
    pub pause_chord: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            daily_new_cards: None,
            wild_mode: WildMode::default(),
            speed: Speed::default(),
            scenario_unlock_days: 7,
            copy_mode_keys: CopyModeKeys::default(),
            option_as_meta: true,
            pause_chord: "C-S-p".to_string(),
        }
    }
}

impl Prefs {
    /// Reads prefs field by field, so one unknown or invalid value costs only
    /// that field: it takes its default and its name is returned. Missing
    /// fields take the default; unknown fields are ignored.
    pub fn from_fields(fields: &Map<String, Value>) -> (Prefs, Vec<&'static str>) {
        let mut p = Prefs::default();
        let mut bad = Vec::new();
        read(fields, "daily_new_cards", &mut p.daily_new_cards, &mut bad);
        read(fields, "wild_mode", &mut p.wild_mode, &mut bad);
        read(fields, "speed", &mut p.speed, &mut bad);
        read(
            fields,
            "scenario_unlock_days",
            &mut p.scenario_unlock_days,
            &mut bad,
        );
        read(fields, "copy_mode_keys", &mut p.copy_mode_keys, &mut bad);
        read(fields, "option_as_meta", &mut p.option_as_meta, &mut bad);
        read(fields, "pause_chord", &mut p.pause_chord, &mut bad);
        (p, bad)
    }
}

fn read<T: DeserializeOwned>(
    fields: &Map<String, Value>,
    name: &'static str,
    slot: &mut T,
    bad: &mut Vec<&'static str>,
) {
    if let Some(v) = fields.get(name) {
        match T::deserialize(v) {
            Ok(x) => *slot = x,
            Err(_) => bad.push(name),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let p = Prefs::default();
        assert_eq!(p.daily_new_cards, None);
        assert_eq!(p.wild_mode, WildMode::Sometimes);
        assert_eq!(p.speed, Speed::Normal);
        assert_eq!(p.scenario_unlock_days, 7);
        assert_eq!(p.copy_mode_keys, CopyModeKeys::Vi);
        assert!(p.option_as_meta);
        assert_eq!(p.pause_chord, "C-S-p");
    }

    #[test]
    fn unknown_and_missing_fields_load() {
        let p: Prefs =
            serde_json::from_str(r#"{"wild_mode": "always", "colour": "teal"}"#).unwrap();
        assert_eq!(
            p,
            Prefs {
                wild_mode: WildMode::Always,
                ..Prefs::default()
            }
        );
    }

    fn fields(json: &str) -> Map<String, Value> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn from_fields_reads_every_field() {
        let p = Prefs {
            daily_new_cards: Some(3),
            wild_mode: WildMode::Off,
            speed: Speed::Brisk,
            scenario_unlock_days: 2,
            copy_mode_keys: CopyModeKeys::Emacs,
            option_as_meta: false,
            pause_chord: "C-q".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(Prefs::from_fields(&fields(&json)), (p, vec![]));
    }

    #[test]
    fn one_bad_value_keeps_every_other_pref() {
        let (p, bad) = Prefs::from_fields(&fields(
            r#"{"speed": "turbo", "wild_mode": "always", "daily_new_cards": -4,
                "pause_chord": "C-q", "colour": "teal"}"#,
        ));
        assert_eq!(
            p,
            Prefs {
                wild_mode: WildMode::Always,
                pause_chord: "C-q".into(),
                ..Prefs::default()
            }
        );
        assert_eq!(bad, vec!["daily_new_cards", "speed"]);
    }

    #[test]
    fn enums_serialize_lowercase() {
        let json = serde_json::to_string(&Prefs {
            copy_mode_keys: CopyModeKeys::Emacs,
            wild_mode: WildMode::Off,
            ..Prefs::default()
        })
        .unwrap();
        assert!(json.contains(r#""copy_mode_keys":"emacs""#), "{json}");
        assert!(json.contains(r#""wild_mode":"off""#), "{json}");
    }
}
