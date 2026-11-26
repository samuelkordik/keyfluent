//! Prefs, progress and the review log on disk (spec §6.8).
//!
//! Three files live in the store's folder: `prefs.json`, `progress.json`
//! (`"schema": 1`) and the append-only `reviews.jsonl`. Whole-file writes are
//! atomic (temp file, sync, rename). A file that can't be read is moved aside
//! as `<name>.bad-<unix-seconds>`, never deleted, and the load says so in its
//! notes. The store never reads the clock: loads take `now`.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use rs_fsrs::State;
use serde::{Deserialize, Serialize};

use crate::grade::{Fail, Grade};
use crate::prefs::Prefs;
use crate::schedule::{new_card, CardProgress};
use crate::speed::Sample;

const PREFS: &str = "prefs.json";
const PROGRESS: &str = "progress.json";
const REVIEWS: &str = "reviews.jsonl";
/// The `progress.json` schema this version writes and reads.
const SCHEMA: u32 = 1;
/// Largest believable review or lapse count (and day count) for one card.
const MAX_COUNT: i64 = 100_000;
/// Attempts at a free `<name>.bad-<ts>[-n]` name before giving up.
const MAX_BAD_SUFFIX: u32 = 1000;

/// One graded attempt, as kept in `reviews.jsonl`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub at: DateTime<Utc>,
    pub pack: String,
    pub card: String,
    pub grade: Grade,
    pub time_ms: u64,
    pub target_ms: u64,
    /// Binding keys the learner pressed.
    pub binding_keys: u32,
    /// Characters the learner typed.
    pub typed_chars: u32,
    /// Binding keys of the card's canonical route.
    pub canonical_keys: u32,
    /// Typed characters of the card's canonical route.
    pub canonical_chars: u32,
    pub fail: Option<Fail>,
    pub capped: bool,
    pub wild: bool,
    /// Always `false` as the app writes the log: intro steps (Show, Guided,
    /// Recall) are ungraded and never logged, only graded reviews are. The
    /// field is kept so a log that does hold intro attempts still reads;
    /// [`samples`] leaves such records out.
    pub intro: bool,
}

/// The speed samples in a review log, oldest first. Intro attempts never
/// count towards the learner's personal speed.
pub fn samples(records: &[ReviewRecord]) -> Vec<Sample> {
    records
        .iter()
        .filter(|r| !r.intro)
        .map(Sample::from)
        .collect()
}

impl From<&ReviewRecord> for Sample {
    fn from(r: &ReviewRecord) -> Sample {
        Sample {
            time: Duration::from_millis(r.time_ms),
            binding_keys: usize::try_from(r.canonical_keys).unwrap_or(usize::MAX),
            typed_chars: usize::try_from(r.canonical_chars).unwrap_or(usize::MAX),
            success: r.grade != Grade::Again,
        }
    }
}

/// Scheduling state for every card the learner has met, by pack then card id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub schema: u32,
    pub packs: BTreeMap<String, BTreeMap<String, CardProgress>>,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            schema: SCHEMA,
            packs: BTreeMap::new(),
        }
    }
}

/// A loaded value and what the learner should be told about loading it.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded<T> {
    pub value: T,
    pub notes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("could not encode {name}: {source}")]
    Encode {
        name: &'static str,
        source: serde_json::Error,
    },
    #[error("progress.json has schema {0}, newer than this version of Keyfluent understands")]
    FutureSchema(u32),
}

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// The folder holding the learner's files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn at(root: PathBuf) -> Store {
        Store { root }
    }

    /// The platform's per-user data folder for Keyfluent, if there is one.
    pub fn default_location() -> Option<Store> {
        directories::ProjectDirs::from("com", "samuelkordik", "Keyfluent")
            .map(|dirs| Store::at(dirs.data_dir().to_path_buf()))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Missing or unreadable prefs load as the defaults.
    pub fn load_prefs(&self, now: DateTime<Utc>) -> Loaded<Prefs> {
        let mut notes = Vec::new();
        let value = match self.read(PREFS) {
            Ok(None) => Prefs::default(),
            Ok(Some(bytes)) => match serde_json::from_slice(&bytes) {
                Ok(prefs) => prefs,
                Err(_) => {
                    notes.push(self.set_aside(PREFS, now, "using defaults"));
                    Prefs::default()
                }
            },
            Err(e) => {
                notes.push(format!("{PREFS} could not be read ({e}); using defaults"));
                Prefs::default()
            }
        };
        Loaded { value, notes }
    }

    pub fn save_prefs(&self, prefs: &Prefs) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(prefs).map_err(|source| StoreError::Encode {
            name: PREFS,
            source,
        })?;
        self.write_atomic(PREFS, &bytes)
    }

    /// Missing progress loads empty; progress whose top level can't be read
    /// is set aside whole and loads empty. A newer schema is refused and the
    /// file left untouched. Cards are read one by one, so one odd card never
    /// costs the rest: a card with out-of-range scheduling data has its
    /// schedule reset; a card whose scheduling data can't be read has it
    /// reset too, and a card that can't be read at all is dropped. In each
    /// case the original file is copied aside, once, so nothing stored is
    /// lost; if that copy fails the load fails (and the caller must not
    /// save over the file).
    pub fn load_progress(&self, now: DateTime<Utc>) -> Result<Loaded<Progress>, StoreError> {
        let Some(bytes) = self.read(PROGRESS)? else {
            return Ok(Loaded {
                value: Progress::default(),
                notes: Vec::new(),
            });
        };
        let parsed = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| {
                let schema = v.get("schema")?.as_u64()?;
                Some((schema, v))
            });
        let packs = match parsed {
            Some((schema, _)) if schema > u64::from(SCHEMA) => {
                return Err(StoreError::FutureSchema(
                    u32::try_from(schema).unwrap_or(u32::MAX),
                ));
            }
            Some((schema, mut v)) if schema == u64::from(SCHEMA) => v
                .get_mut("packs")
                .map(serde_json::Value::take)
                .and_then(|p| serde_json::from_value::<RawPacks>(p).ok()),
            _ => None,
        };
        let Some(packs) = packs else {
            let note = self.try_set_aside(PROGRESS, now, "starting fresh")?;
            return Ok(Loaded {
                value: Progress::default(),
                notes: vec![note],
            });
        };
        let mut progress = Progress::default();
        let mut notes = Vec::new();
        let mut lost = false;
        for (pack, cards) in packs {
            let kept = progress.packs.entry(pack.clone()).or_default();
            for (card, raw) in cards {
                match read_card(raw, now) {
                    CardRead::Ok(p) if plausible(&p) => {
                        kept.insert(card, p);
                    }
                    CardRead::Ok(mut p) => {
                        lost = true;
                        p.fsrs = new_card(now);
                        notes.push(format!(
                            "{PROGRESS}: card {pack}/{card} had out-of-range scheduling data; \
                             its schedule was reset"
                        ));
                        kept.insert(card, p);
                    }
                    CardRead::ScheduleUnreadable(p) => {
                        lost = true;
                        notes.push(format!(
                            "{PROGRESS}: card {pack}/{card} had unreadable scheduling data; \
                             its schedule was reset"
                        ));
                        kept.insert(card, p);
                    }
                    CardRead::Unreadable => {
                        lost = true;
                        notes.push(format!(
                            "{PROGRESS}: card {pack}/{card} was unreadable and was left out"
                        ));
                    }
                }
            }
        }
        if lost {
            notes.push(self.copy_aside(PROGRESS, now)?);
        }
        Ok(Loaded {
            value: progress,
            notes,
        })
    }

    pub fn save_progress(&self, progress: &Progress) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(progress).map_err(|source| StoreError::Encode {
            name: PROGRESS,
            source,
        })?;
        self.write_atomic(PROGRESS, &bytes)
    }

    /// Add one record to the end of `reviews.jsonl`.
    pub fn append_review(&self, record: &ReviewRecord) -> Result<(), StoreError> {
        let mut line = serde_json::to_vec(record).map_err(|source| StoreError::Encode {
            name: REVIEWS,
            source,
        })?;
        line.push(b'\n');
        fs::create_dir_all(&self.root).map_err(io_err(&self.root))?;
        let path = self.root.join(REVIEWS);
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .map_err(io_err(&path))?;
        // A torn last line (a crash mid-write) must not swallow this record.
        if !ends_with_newline_or_empty(&mut file).map_err(io_err(&path))? {
            line.insert(0, b'\n');
        }
        file.write_all(&line).map_err(io_err(&path))
    }

    /// Every readable record in `reviews.jsonl`, oldest first; unreadable
    /// lines are skipped and counted.
    pub fn load_reviews(&self) -> Loaded<Vec<ReviewRecord>> {
        let bytes = match self.read(REVIEWS) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                return Loaded {
                    value: Vec::new(),
                    notes: Vec::new(),
                }
            }
            Err(e) => {
                return Loaded {
                    value: Vec::new(),
                    notes: vec![format!("{REVIEWS} could not be read ({e})")],
                }
            }
        };
        let mut value = Vec::new();
        let mut skipped = 0usize;
        for line in bytes.split(|&b| b == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match serde_json::from_slice(line) {
                Ok(record) => value.push(record),
                Err(_) => skipped += 1,
            }
        }
        let notes = match skipped {
            0 => Vec::new(),
            1 => vec![format!("skipped 1 unreadable line in {REVIEWS}")],
            n => vec![format!("skipped {n} unreadable lines in {REVIEWS}")],
        };
        Loaded { value, notes }
    }

    /// The file's bytes, or `None` if it doesn't exist.
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.root.join(name);
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err(&path)(e)),
        }
    }

    /// Move an unreadable file aside and describe what happened.
    fn try_set_aside(
        &self,
        name: &str,
        now: DateTime<Utc>,
        outcome: &str,
    ) -> Result<String, StoreError> {
        let from = self.root.join(name);
        let kept = self.free_bad_name(name, now)?;
        fs::rename(&from, self.root.join(&kept)).map_err(io_err(&from))?;
        Ok(format!("{name} was unreadable; kept as {kept}; {outcome}"))
    }

    /// Copy a file aside (it stays in place) and describe what happened.
    fn copy_aside(&self, name: &str, now: DateTime<Utc>) -> Result<String, StoreError> {
        let from = self.root.join(name);
        let kept = self.free_bad_name(name, now)?;
        fs::copy(&from, self.root.join(&kept)).map_err(io_err(&from))?;
        Ok(format!("{name}: the original file was kept as {kept}"))
    }

    /// The first unused `<name>.bad-<ts>[-n]` in the store's folder.
    fn free_bad_name(&self, name: &str, now: DateTime<Utc>) -> Result<String, StoreError> {
        let base = format!("{name}.bad-{}", now.timestamp());
        let mut kept = base.clone();
        let mut n = 0;
        while self.root.join(&kept).exists() {
            n += 1;
            if n > MAX_BAD_SUFFIX {
                return Err(StoreError::Io {
                    path: self.root.join(name),
                    source: io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "no free name to set the unreadable file aside",
                    ),
                });
            }
            kept = format!("{base}-{n}");
        }
        Ok(kept)
    }

    /// Like [`Store::try_set_aside`], but a failure becomes the note.
    fn set_aside(&self, name: &str, now: DateTime<Utc>, outcome: &str) -> String {
        self.try_set_aside(name, now, outcome).unwrap_or_else(|e| {
            format!("{name} was unreadable and could not be set aside ({e}); {outcome}")
        })
    }

    /// Write `name` so that it holds either the old or the new bytes, never
    /// a mix: temp file in the same folder, sync, rename over.
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root).map_err(io_err(&self.root))?;
        let tmp = self.root.join(format!("{name}.tmp-{}", std::process::id()));
        let target = self.root.join(name);
        let result = (|| {
            let mut file = File::create(&tmp).map_err(io_err(&tmp))?;
            file.write_all(bytes).map_err(io_err(&tmp))?;
            file.sync_all().map_err(io_err(&tmp))?;
            drop(file);
            fs::rename(&tmp, &target).map_err(io_err(&target))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }
}

fn ends_with_newline_or_empty(file: &mut File) -> io::Result<bool> {
    if file.metadata()?.len() == 0 {
        return Ok(true);
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last)?;
    Ok(last == *b"\n")
}

/// `progress.json`'s packs, each card still unparsed.
type RawPacks = BTreeMap<String, BTreeMap<String, serde_json::Value>>;

/// One card read from `progress.json`.
enum CardRead {
    Ok(CardProgress),
    /// Everything but the FSRS state read; the schedule is reset.
    ScheduleUnreadable(CardProgress),
    Unreadable,
}

fn read_card(mut raw: serde_json::Value, now: DateTime<Utc>) -> CardRead {
    if let Ok(p) = serde_json::from_value::<CardProgress>(raw.clone()) {
        return CardRead::Ok(p);
    }
    let Ok(fresh) = serde_json::to_value(new_card(now)) else {
        return CardRead::Unreadable;
    };
    match raw.get_mut("fsrs") {
        Some(fsrs) => *fsrs = fresh,
        None => return CardRead::Unreadable,
    }
    match serde_json::from_value::<CardProgress>(raw) {
        Ok(p) => CardRead::ScheduleUnreadable(p),
        Err(_) => CardRead::Unreadable,
    }
}

/// Whether a card's FSRS state is safe to hand to the scheduler (ruling 7).
/// Past New, FSRS divides by stability and difficulty must be on its 1–10
/// scale, so those are held to sane ranges too.
fn plausible(p: &CardProgress) -> bool {
    let (Some(earliest), Some(latest)) = (
        Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).single(),
        Utc.with_ymd_and_hms(2200, 1, 1, 0, 0, 0).single(),
    ) else {
        return false;
    };
    let date_ok = |d: DateTime<Utc>| (earliest..latest).contains(&d);
    let real_ok = |x: f64| x.is_finite() && x >= 0.0;
    let count_ok = |n: i64| (0..=MAX_COUNT).contains(&n);
    let c = &p.fsrs;
    let scheduled_ok =
        c.state == State::New || (c.stability >= 0.01 && (1.0..=10.0).contains(&c.difficulty));
    scheduled_ok
        && date_ok(c.due)
        && date_ok(c.last_review)
        && real_ok(c.stability)
        && real_ok(c.difficulty)
        && count_ok(i64::from(c.reps))
        && count_ok(i64::from(c.lapses))
        && count_ok(c.elapsed_days)
        && count_ok(c.scheduled_days)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;
    use tempfile::TempDir;

    use super::*;
    use crate::grade::{Fail, Graded, Speed};
    use crate::prefs::WildMode;
    use crate::schedule::Scheduler;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 26, 12, 0, 0).unwrap()
    }

    fn store() -> (TempDir, Store) {
        let dir = TempDir::new().unwrap();
        let store = Store::at(dir.path().join("data"));
        (dir, store)
    }

    fn names(store: &Store) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(&store.root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn sample_progress() -> Progress {
        let graded = Graded {
            grade: Grade::Good,
            time: Duration::from_millis(2345),
            target: Duration::from_secs(3),
            binding_keys: 2,
            typed_chars: 0,
            canonical_keys: 2,
            canonical_chars: 0,
            fail: None,
            capped: false,
            practiced: Vec::new(),
        };
        let (card, _) = Scheduler::new().review(None, &graded, now());
        let mut progress = Progress::default();
        progress
            .packs
            .entry("tmux".into())
            .or_default()
            .insert("split-horizontal".into(), card);
        progress
    }

    fn record(card: &str) -> ReviewRecord {
        ReviewRecord {
            at: now(),
            pack: "tmux".into(),
            card: card.into(),
            grade: Grade::Again,
            time_ms: 4100,
            target_ms: 2000,
            binding_keys: 7,
            typed_chars: 3,
            canonical_keys: 2,
            canonical_chars: 1,
            fail: Some(Fail::TooManyKeys),
            capped: false,
            wild: true,
            intro: false,
        }
    }

    #[test]
    fn missing_files_load_defaults_without_notes() {
        let (_d, s) = store();
        let prefs = s.load_prefs(now());
        assert_eq!(prefs.value, Prefs::default());
        assert!(prefs.notes.is_empty());
        let progress = s.load_progress(now()).unwrap();
        assert_eq!(progress.value, Progress::default());
        assert_eq!(progress.value.schema, 1);
        assert!(progress.notes.is_empty());
        let reviews = s.load_reviews();
        assert!(reviews.value.is_empty());
        assert!(reviews.notes.is_empty());
    }

    #[test]
    fn prefs_round_trip() {
        let (_d, s) = store();
        let prefs = Prefs {
            daily_new_cards: Some(5),
            wild_mode: WildMode::Always,
            speed: Speed::Brisk,
            option_as_meta: false,
            ..Prefs::default()
        };
        s.save_prefs(&prefs).unwrap();
        let back = s.load_prefs(now());
        assert_eq!(back.value, prefs);
        assert!(back.notes.is_empty());
    }

    #[test]
    fn prefs_with_unknown_and_missing_fields_load() {
        let (_d, s) = store();
        fs::create_dir_all(&s.root).unwrap();
        fs::write(
            s.root.join("prefs.json"),
            r#"{"speed": "relaxed", "future_option": [1, 2]}"#,
        )
        .unwrap();
        let p = s.load_prefs(now());
        assert_eq!(p.value.speed, Speed::Relaxed);
        assert_eq!(p.value.scenario_unlock_days, 7);
        assert!(p.notes.is_empty());
    }

    #[test]
    fn corrupt_prefs_are_kept_aside() {
        let (_d, s) = store();
        fs::create_dir_all(&s.root).unwrap();
        fs::write(s.root.join("prefs.json"), "{\"speed\": ").unwrap();
        let p = s.load_prefs(now());
        assert_eq!(p.value, Prefs::default());
        let ts = now().timestamp();
        assert_eq!(names(&s), vec![format!("prefs.json.bad-{ts}")]);
        assert_eq!(
            p.notes,
            vec![format!(
                "prefs.json was unreadable; kept as prefs.json.bad-{ts}; using defaults"
            )]
        );
    }

    #[test]
    fn progress_round_trip() {
        let (_d, s) = store();
        let progress = sample_progress();
        s.save_progress(&progress).unwrap();
        let back = s.load_progress(now()).unwrap();
        assert_eq!(back.value, progress);
        assert!(back.notes.is_empty());
        let text = fs::read_to_string(s.root.join("progress.json")).unwrap();
        assert!(text.contains("\"schema\": 1"), "{text}");
    }

    #[test]
    fn truncated_progress_starts_fresh_and_keeps_the_original() {
        let (_d, s) = store();
        s.save_progress(&sample_progress()).unwrap();
        let path = s.root.join("progress.json");
        let full = fs::read_to_string(&path).unwrap();
        let truncated = &full[..full.len() / 2];
        fs::write(&path, truncated).unwrap();

        let loaded = s.load_progress(now()).unwrap();
        assert_eq!(loaded.value, Progress::default());
        let ts = now().timestamp();
        assert_eq!(
            loaded.notes,
            vec![format!(
                "progress.json was unreadable; kept as progress.json.bad-{ts}; starting fresh"
            )]
        );
        let kept = s.root.join(format!("progress.json.bad-{ts}"));
        assert_eq!(fs::read_to_string(kept).unwrap(), truncated);
        assert!(!path.exists());
    }

    #[test]
    fn bad_names_never_overwrite_earlier_ones() {
        let (_d, s) = store();
        fs::create_dir_all(&s.root).unwrap();
        let path = s.root.join("progress.json");
        for _ in 0..3 {
            fs::write(&path, "not json").unwrap();
            s.load_progress(now()).unwrap();
        }
        let ts = now().timestamp();
        assert_eq!(
            names(&s),
            vec![
                format!("progress.json.bad-{ts}"),
                format!("progress.json.bad-{ts}-1"),
                format!("progress.json.bad-{ts}-2"),
            ]
        );
    }

    #[test]
    fn future_schema_is_refused_and_left_alone() {
        let (_d, s) = store();
        fs::create_dir_all(&s.root).unwrap();
        let path = s.root.join("progress.json");
        let text = r#"{"schema": 2, "packs": {}, "new_thing": true}"#;
        fs::write(&path, text).unwrap();
        let result = s.load_progress(now());
        assert!(
            matches!(result, Err(StoreError::FutureSchema(2))),
            "{result:?}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        assert_eq!(names(&s), vec!["progress.json".to_string()]);
    }

    #[test]
    fn out_of_range_cards_are_reset_with_a_note() {
        let (_d, s) = store();
        let mut progress = sample_progress();
        let cards = progress.packs.get_mut("tmux").unwrap();
        let good = cards.get("split-horizontal").unwrap().clone();
        cards.insert("big-reps".into(), good.clone());
        cards.insert("negative-stability".into(), good.clone());
        cards.insert("far-due".into(), good.clone());
        cards.insert("zero-stability".into(), good.clone());
        cards.insert("zero-difficulty".into(), good.clone());
        s.save_progress(&progress).unwrap();

        // Hand-edit the file the way a curious learner might.
        let path = s.root.join("progress.json");
        let mut json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let tmux = &mut json["packs"]["tmux"];
        tmux["big-reps"]["fsrs"]["reps"] = 2147483647.into();
        tmux["negative-stability"]["fsrs"]["stability"] = (-1.0).into();
        tmux["far-due"]["fsrs"]["due"] = "9999-01-01T00:00:00Z".into();
        tmux["zero-stability"]["fsrs"]["state"] = "Review".into();
        tmux["zero-stability"]["fsrs"]["stability"] = 0.0.into();
        tmux["zero-difficulty"]["fsrs"]["state"] = "Review".into();
        tmux["zero-difficulty"]["fsrs"]["difficulty"] = 0.0.into();
        let edited = serde_json::to_string(&json).unwrap();
        fs::write(&path, &edited).unwrap();

        let loaded = s.load_progress(now()).unwrap();
        let cards = &loaded.value.packs["tmux"];
        assert_eq!(cards["split-horizontal"], good);
        for id in [
            "big-reps",
            "far-due",
            "negative-stability",
            "zero-difficulty",
            "zero-stability",
        ] {
            let c = &cards[id];
            assert_eq!(c.mastery(), crate::schedule::Mastery::New, "{id}");
            assert_eq!(c.fsrs.reps, 0, "{id}");
            assert_eq!(c.fsrs.due, now(), "{id}");
            assert_eq!(c.fsrs.last_review, now(), "{id}");
            assert_eq!(c.reviews, good.reviews, "{id}");
            assert_eq!(c.best, good.best, "{id}");
            assert_eq!(c.last_times, good.last_times, "{id}");
        }
        assert_eq!(
            loaded.notes,
            vec![
                "progress.json: card tmux/big-reps had out-of-range scheduling data; its schedule was reset".to_string(),
                "progress.json: card tmux/far-due had out-of-range scheduling data; its schedule was reset".to_string(),
                "progress.json: card tmux/negative-stability had out-of-range scheduling data; its schedule was reset".to_string(),
                "progress.json: card tmux/zero-difficulty had out-of-range scheduling data; its schedule was reset".to_string(),
                "progress.json: card tmux/zero-stability had out-of-range scheduling data; its schedule was reset".to_string(),
                format!("progress.json: the original file was kept as progress.json.bad-{}", now().timestamp()),
            ]
        );
        let kept = s
            .root
            .join(format!("progress.json.bad-{}", now().timestamp()));
        assert_eq!(fs::read_to_string(kept).unwrap(), edited);
        // The reset cards schedule normally afterwards.
        let graded = Graded {
            grade: Grade::Good,
            time: Duration::from_secs(2),
            target: Duration::from_secs(3),
            binding_keys: 2,
            typed_chars: 0,
            canonical_keys: 2,
            canonical_chars: 0,
            fail: None,
            capped: false,
            practiced: Vec::new(),
        };
        let (next, _) = Scheduler::new().review(Some(&cards["big-reps"]), &graded, now());
        assert!(next.due() > now());
    }

    fn graded(grade: Grade) -> Graded {
        Graded {
            grade,
            time: Duration::from_secs(2),
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

    /// A Review-state card with the given stability and difficulty.
    fn review_card(stability: f64, difficulty: f64) -> CardProgress {
        let mut card = sample_progress().packs["tmux"]["split-horizontal"].clone();
        card.fsrs.state = State::Review;
        card.fsrs.stability = stability;
        card.fsrs.difficulty = difficulty;
        card
    }

    #[test]
    fn odd_review_cards_never_cost_the_progress_file() {
        for (stability, difficulty, grade) in [(0.0, 5.0, Grade::Good), (5.0, 0.0, Grade::Again)] {
            let (_d, s) = store();
            let mut progress = sample_progress();
            let odd = review_card(stability, difficulty);
            let later = now() + chrono::Duration::days(3);
            let (next, _) = Scheduler::new().review(Some(&odd), &graded(grade), later);
            progress
                .packs
                .get_mut("tmux")
                .unwrap()
                .insert("odd".into(), next.clone());
            s.save_progress(&progress).unwrap();

            let back = s.load_progress(later).unwrap();
            assert!(back.notes.is_empty(), "{:?}", back.notes);
            assert_eq!(back.value, progress);
            assert_eq!(back.value.packs["tmux"].len(), 2);
            assert_eq!(names(&s), vec!["progress.json".to_string()]);
        }
    }

    #[test]
    fn one_unreadable_card_spares_the_others_and_is_kept_aside() {
        let (_d, s) = store();
        let mut progress = sample_progress();
        let good = progress.packs["tmux"]["split-horizontal"].clone();
        let cards = progress.packs.get_mut("tmux").unwrap();
        cards.insert("bad-reps".into(), good.clone());
        cards.insert("garbage".into(), good.clone());
        cards.insert("zoom".into(), good.clone());
        s.save_progress(&progress).unwrap();

        let path = s.root.join("progress.json");
        let mut json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        json["packs"]["tmux"]["bad-reps"]["fsrs"]["reps"] = "x".into();
        json["packs"]["tmux"]["garbage"] = "not a card".into();
        let edited = serde_json::to_string(&json).unwrap();
        fs::write(&path, &edited).unwrap();

        let loaded = s.load_progress(now()).unwrap();
        let cards = &loaded.value.packs["tmux"];
        assert_eq!(cards["split-horizontal"], good);
        assert_eq!(cards["zoom"], good);
        assert!(!cards.contains_key("garbage"));
        // The card's own history survives; only its schedule restarts.
        let reset = &cards["bad-reps"];
        assert_eq!(reset.fsrs, new_card(now()));
        assert_eq!(
            (reset.reviews, reset.best, &reset.last_times),
            (good.reviews, good.best, &good.last_times)
        );
        let ts = now().timestamp();
        assert_eq!(
            loaded.notes,
            vec![
                "progress.json: card tmux/bad-reps had unreadable scheduling data; its schedule was reset".to_string(),
                "progress.json: card tmux/garbage was unreadable and was left out".to_string(),
                format!("progress.json: the original file was kept as progress.json.bad-{ts}"),
            ]
        );
        // The original stays in place too, untouched until the next save.
        assert_eq!(fs::read_to_string(&path).unwrap(), edited);
        let kept = s.root.join(format!("progress.json.bad-{ts}"));
        assert_eq!(fs::read_to_string(kept).unwrap(), edited);
    }

    #[test]
    fn unreadable_packs_set_the_file_aside() {
        let (_d, s) = store();
        fs::create_dir_all(&s.root).unwrap();
        fs::write(
            s.root.join("progress.json"),
            r#"{"schema": 1, "packs": [1]}"#,
        )
        .unwrap();
        let loaded = s.load_progress(now()).unwrap();
        assert_eq!(loaded.value, Progress::default());
        let ts = now().timestamp();
        assert_eq!(names(&s), vec![format!("progress.json.bad-{ts}")]);
    }

    #[test]
    fn save_progress_leaves_no_temp_file() {
        let (_d, s) = store();
        s.save_progress(&sample_progress()).unwrap();
        s.save_progress(&Progress::default()).unwrap();
        assert_eq!(names(&s), vec!["progress.json".to_string()]);
        assert_eq!(s.load_progress(now()).unwrap().value, Progress::default());
    }

    #[cfg(unix)]
    #[test]
    fn failed_save_keeps_the_previous_progress() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, s) = store();
        let progress = sample_progress();
        s.save_progress(&progress).unwrap();
        fs::set_permissions(&s.root, fs::Permissions::from_mode(0o555)).unwrap();
        // Root ignores directory permissions; the case can't be staged then.
        let probe = s.root.join("probe");
        if fs::write(&probe, "x").is_ok() {
            fs::remove_file(&probe).unwrap();
            fs::set_permissions(&s.root, fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }
        let result = s.save_progress(&Progress::default());
        fs::set_permissions(&s.root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(result, Err(StoreError::Io { .. })), "{result:?}");
        assert_eq!(names(&s), vec!["progress.json".to_string()]);
        assert_eq!(s.load_progress(now()).unwrap().value, progress);
    }

    #[test]
    fn reviews_skip_unreadable_lines() {
        let (_d, s) = store();
        for card in ["a", "b", "c"] {
            s.append_review(&record(card)).unwrap();
        }
        let path = s.root.join("reviews.jsonl");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"at\": \"garbage\n").unwrap();
        drop(f);
        s.append_review(&record("d")).unwrap();

        let loaded = s.load_reviews();
        let cards: Vec<&str> = loaded.value.iter().map(|r| r.card.as_str()).collect();
        assert_eq!(cards, ["a", "b", "c", "d"]);
        assert_eq!(loaded.value[0], record("a"));
        assert_eq!(
            loaded.notes,
            vec!["skipped 1 unreadable line in reviews.jsonl".to_string()]
        );
    }

    #[test]
    fn append_after_a_torn_last_line_starts_a_new_line() {
        let (_d, s) = store();
        s.append_review(&record("a")).unwrap();
        let path = s.root.join("reviews.jsonl");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"at\": \"2026-").unwrap();
        drop(f);
        s.append_review(&record("b")).unwrap();
        s.append_review(&record("c")).unwrap();
        let loaded = s.load_reviews();
        let cards: Vec<&str> = loaded.value.iter().map(|r| r.card.as_str()).collect();
        assert_eq!(cards, ["a", "b", "c"]);
        assert_eq!(
            loaded.notes,
            vec!["skipped 1 unreadable line in reviews.jsonl".to_string()]
        );
    }

    #[test]
    fn review_record_json_shape() {
        let json = serde_json::to_string(&record("a")).unwrap();
        for part in [
            r#""grade":"again""#,
            r#""fail":"too_many_keys""#,
            r#""time_ms":4100"#,
            r#""at":"2026-11-26T12:00:00Z""#,
        ] {
            assert!(json.contains(part), "{part} in {json}");
        }
        let back: ReviewRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record("a"));
    }

    #[test]
    fn review_record_becomes_a_sample_with_canonical_counts() {
        let sample = Sample::from(&record("a"));
        assert_eq!(
            sample,
            Sample {
                time: Duration::from_millis(4100),
                binding_keys: 2,
                typed_chars: 1,
                success: false,
            }
        );
        let ok = ReviewRecord {
            grade: Grade::Hard,
            fail: None,
            ..record("a")
        };
        assert!(Sample::from(&ok).success);
    }

    #[test]
    fn samples_leave_out_intro_records() {
        let intro = ReviewRecord {
            intro: true,
            time_ms: 9999,
            ..record("b")
        };
        let ok = ReviewRecord {
            grade: Grade::Good,
            fail: None,
            ..record("c")
        };
        let got = samples(&[record("a"), intro, ok.clone()]);
        assert_eq!(got, vec![Sample::from(&record("a")), Sample::from(&ok)]);
    }

    #[test]
    fn default_location_is_a_keyfluent_folder() {
        if let Some(s) = Store::default_location() {
            assert!(s
                .root
                .to_string_lossy()
                .to_lowercase()
                .contains("keyfluent"));
        }
    }
}
