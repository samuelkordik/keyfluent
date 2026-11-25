//! Card model and TOML loading. Cards are tied to goals, not keys.

use crate::key::{parse_keys, Key, KeyParseError};
use serde::de::{DeserializeOwned, Deserializer, Error as _};
use serde::Deserialize;
use std::collections::HashSet;

/// One step of a canonical answer: binding keys, or literal typed text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Keys(Vec<Key>),
    Text(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawStep {
    Keys(String),
    Text { text: String },
}

impl Step {
    fn from_raw(raw: RawStep) -> Result<Step, KeyParseError> {
        match raw {
            RawStep::Keys(s) => {
                let keys = parse_keys(&s)?;
                if keys.is_empty() {
                    return Err(KeyParseError::Empty);
                }
                Ok(Step::Keys(keys))
            }
            RawStep::Text { text } => Ok(Step::Text(text)),
        }
    }
}

impl<'de> Deserialize<'de> for Step {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Step::from_raw(RawStep::deserialize(d)?).map_err(D::Error::custom)
    }
}

/// A flash card, generic over the pack's goal type.
#[derive(Clone, Debug, PartialEq)]
pub struct Card<G> {
    pub id: String,
    pub category: String,
    pub intent: String,
    pub teach: String,
    pub start: String,
    pub goal: G,
    pub canonical: Vec<Step>,
    pub requires: Vec<String>,
    pub components: Vec<String>,
}

impl<G> Card<G> {
    /// Number of binding keys (keys in `Keys` steps), per the grading rule.
    pub fn binding_key_count(&self) -> usize {
        self.canonical
            .iter()
            .map(|s| match s {
                Step::Keys(k) => k.len(),
                Step::Text(_) => 0,
            })
            .sum()
    }

    /// Number of typed characters (chars of `Text` steps).
    pub fn typed_char_count(&self) -> usize {
        self.canonical
            .iter()
            .map(|s| match s {
                Step::Text(t) => t.chars().count(),
                Step::Keys(_) => 0,
            })
            .sum()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CardError {
    #[error("invalid card file: {0}")]
    Toml(String),
    #[error("duplicate card id {0:?}")]
    DuplicateId(String),
    #[error("card {id:?}, field `canonical`, step {step}: {reason}")]
    BadStep {
        id: String,
        step: usize,
        reason: String,
    },
    #[error("card {id:?}, field `canonical`, step {step}: {source}")]
    BadKey {
        id: String,
        step: usize,
        source: KeyParseError,
    },
    #[error("card {id:?}, field `goal`: {reason}")]
    BadGoal { id: String, reason: String },
    #[error("card {id:?}: {reason}")]
    Field { id: String, reason: String },
}

#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    card: Vec<toml::Table>,
}

fn take_string(
    t: &mut toml::Table,
    id: &str,
    name: &str,
    optional: bool,
) -> Result<String, CardError> {
    match t.remove(name) {
        Some(toml::Value::String(s)) => Ok(s),
        Some(v) => Err(CardError::Field {
            id: id.to_string(),
            reason: format!("field `{name}` must be a string, found {}", v.type_str()),
        }),
        None if optional => Ok(String::new()),
        None => Err(CardError::Field {
            id: id.to_string(),
            reason: format!("missing field `{name}`"),
        }),
    }
}

fn take_list(t: &mut toml::Table, id: &str, name: &str) -> Result<Vec<String>, CardError> {
    match t.remove(name) {
        None => Ok(Vec::new()),
        Some(v) => v.try_into().map_err(|e: toml::de::Error| CardError::Field {
            id: id.to_string(),
            reason: format!("field `{name}`: {}", e.message()),
        }),
    }
}

/// Loads a `[[card]]` TOML file. Errors name the card id and field.
pub fn load_cards<G: DeserializeOwned>(toml_src: &str) -> Result<Vec<Card<G>>, CardError> {
    let file: RawFile = toml::from_str(toml_src).map_err(|e| CardError::Toml(e.to_string()))?;
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(file.card.len());
    for (n, mut t) in file.card.into_iter().enumerate() {
        let id = match t.remove("id") {
            Some(toml::Value::String(s)) => s,
            _ => {
                return Err(CardError::Field {
                    id: format!("card #{}", n + 1),
                    reason: "missing string field `id`".into(),
                })
            }
        };
        if !seen.insert(id.clone()) {
            return Err(CardError::DuplicateId(id));
        }
        let steps = match t.remove("canonical") {
            Some(toml::Value::Array(a)) => a,
            Some(_) => {
                return Err(CardError::Field {
                    id,
                    reason: "field `canonical` must be an array".into(),
                })
            }
            None => {
                return Err(CardError::Field {
                    id,
                    reason: "missing field `canonical`".into(),
                })
            }
        };
        let mut canonical = Vec::with_capacity(steps.len());
        for (i, v) in steps.into_iter().enumerate() {
            let raw: RawStep = v
                .try_into()
                .map_err(|e: toml::de::Error| CardError::BadStep {
                    id: id.clone(),
                    step: i,
                    reason: format!(
                        "expected a key string or {{ text = \"...\" }}: {}",
                        e.message()
                    ),
                })?;
            canonical.push(Step::from_raw(raw).map_err(|source| CardError::BadKey {
                id: id.clone(),
                step: i,
                source,
            })?);
        }
        let goal = t
            .remove("goal")
            .ok_or_else(|| CardError::BadGoal {
                id: id.clone(),
                reason: "missing".into(),
            })?
            .try_into()
            .map_err(|e: toml::de::Error| CardError::BadGoal {
                id: id.clone(),
                reason: e.message().to_string(),
            })?;
        let category = take_string(&mut t, &id, "category", false)?;
        let intent = take_string(&mut t, &id, "intent", false)?;
        let teach = take_string(&mut t, &id, "teach", true)?;
        let start = take_string(&mut t, &id, "start", false)?;
        let requires = take_list(&mut t, &id, "requires")?;
        let components = take_list(&mut t, &id, "components")?;
        if let Some(extra) = t.keys().next() {
            return Err(CardError::Field {
                reason: format!("unknown field `{extra}`"),
                id,
            });
        }
        out.push(Card {
            id,
            category,
            intent,
            teach,
            start,
            goal,
            canonical,
            requires,
            components,
        });
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    const TWO: &str = r#"
[[card]]
id = "split-h"
category = "panes"
intent = "Split the pane"
teach = "Prefix then %"
start = "one-pane"
goal = { panes = 2 }
canonical = ["C-b %"]

[[card]]
id = "rename"
category = "windows"
intent = "Rename window"
teach = "t"
start = "one-pane"
goal = { name = "logs" }
canonical = ["C-b ,", { text = "logs" }, "Enter"]
requires = ["split-h"]
components = ["prefix"]
"#;

    #[test]
    fn loads_two_cards() {
        let cards: Vec<Card<toml::Value>> = load_cards(TWO).unwrap();
        assert_eq!(cards.len(), 2);
        let c = &cards[0];
        assert_eq!(c.id, "split-h");
        assert_eq!(c.category, "panes");
        assert_eq!(c.intent, "Split the pane");
        assert_eq!(c.teach, "Prefix then %");
        assert_eq!(c.start, "one-pane");
        assert_eq!(c.goal["panes"].as_integer(), Some(2));
        assert!(c.requires.is_empty() && c.components.is_empty());
        match c.canonical.as_slice() {
            [Step::Keys(k)] => assert_eq!(k.len(), 2),
            other => panic!("unexpected {other:?}"),
        }
        let r = &cards[1];
        assert_eq!(r.canonical[1], Step::Text("logs".into()));
        assert_eq!(r.requires, ["split-h"]);
        assert_eq!(r.components, ["prefix"]);
    }

    #[test]
    fn duplicate_id_is_error() {
        let src = r#"
[[card]]
id = "x"
category = "c"
intent = "i"
start = "s"
goal = {}
canonical = ["C-b %"]
[[card]]
id = "x"
category = "c"
intent = "i"
start = "s"
goal = {}
canonical = ["C-b %"]
"#;
        let e = load_cards::<toml::Value>(src).unwrap_err();
        assert!(matches!(e, CardError::DuplicateId(ref id) if id == "x"));
    }

    #[test]
    fn unknown_key_names_card() {
        let src = r#"
[[card]]
id = "bad"
category = "c"
intent = "i"
start = "s"
goal = {}
canonical = ["C-b Nonsense"]
"#;
        let e = load_cards::<toml::Value>(src).unwrap_err();
        assert!(matches!(e, CardError::BadKey { .. }));
        assert!(e.to_string().contains("bad"));
    }

    #[test]
    fn unknown_table_shape_names_card() {
        let src = r#"
[[card]]
id = "odd"
category = "c"
intent = "i"
start = "s"
goal = {}
canonical = [{ txt = "a" }]
"#;
        let e = load_cards::<toml::Value>(src).unwrap_err();
        assert!(matches!(e, CardError::BadStep { .. }));
        assert!(e.to_string().contains("odd"));
    }

    #[test]
    fn counts_binding_keys_and_typed_chars() {
        let src = r#"
[[card]]
id = "n"
category = "c"
intent = "i"
start = "s"
goal = {}
canonical = ["C-b ,", "BSpace BSpace BSpace BSpace", { text = "logs" }, "Enter"]
"#;
        let cards: Vec<Card<toml::Value>> = load_cards(src).unwrap();
        assert_eq!(cards[0].binding_key_count(), 7);
        assert_eq!(cards[0].typed_char_count(), 4);
    }

    fn card_with(extra: &str) -> String {
        format!(
            "[[card]]\nid = \"c\"\ncategory = \"c\"\nintent = \"i\"\nstart = \"s\"\ngoal = {{}}\ncanonical = [\"C-b %\"]\n{extra}\n"
        )
    }

    fn err_text(src: &str) -> String {
        load_cards::<toml::Value>(src).unwrap_err().to_string()
    }

    #[test]
    fn bad_requires_type_names_field() {
        let e = err_text(&card_with("requires = \"split-h\""));
        assert!(e.contains("\"c\"") && e.contains("`requires`"), "{e}");
    }

    #[test]
    fn wrong_type_vs_missing_field() {
        let src = card_with("").replace("intent = \"i\"", "intent = 5");
        let e = err_text(&src);
        assert!(e.contains("`intent` must be a string"), "{e}");
        let src = card_with("").replace("intent = \"i\"\n", "");
        let e = err_text(&src);
        assert!(e.contains("missing field `intent`"), "{e}");
    }

    #[test]
    fn empty_key_string_is_error() {
        let src = card_with("").replace("[\"C-b %\"]", "[\"\"]");
        let e = load_cards::<toml::Value>(&src).unwrap_err();
        assert!(matches!(e, CardError::BadKey { .. }), "{e}");
        assert!(e.to_string().contains("\"c\""));
    }

    #[test]
    fn unknown_field_is_rejected() {
        let e = err_text(&card_with("requries = [\"x\"]"));
        assert!(e.contains("\"c\"") && e.contains("`requries`"), "{e}");
    }

    #[test]
    fn missing_id_reports_position() {
        let src = format!("{}\n[[card]]\ncategory = \"c\"\n", card_with(""));
        let e = err_text(&src);
        assert!(e.contains("card #2"), "{e}");
    }

    #[test]
    fn step_deserialize_path() {
        #[derive(Deserialize)]
        struct W {
            s: Vec<Step>,
        }
        let w: W = toml::from_str("s = [\"C-b %\", { text = \"ab\" }]").unwrap();
        assert!(matches!(w.s.first(), Some(Step::Keys(k)) if k.len() == 2));
        assert_eq!(w.s.get(1), Some(&Step::Text("ab".into())));
        assert!(toml::from_str::<W>("s = [\"\"]").is_err());
        assert!(toml::from_str::<W>("s = [{ txt = \"a\" }]").is_err());
    }
}
