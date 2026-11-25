//! The simulated tmux client: key dispatch, the prefix, `-r` repeat and
//! prompts.

use std::time::Duration;

use kf_core::bindings::{Binding, CommandLine, KeyTables};
use kf_core::grid::CellGrid;
use kf_core::key::{Key, KeyBase};
use kf_core::{KeyOutcome, RanBinding};

use crate::commands::{run, Effect, PANE_AREA};
use crate::prompt::{inserts_text, Prompt, PromptStep};
use crate::render::{render, Overlay};
use crate::state::Server;

/// tmux's default `repeat-time`: after a `-r` binding, further `-r` keys of
/// the same table run without the prefix for this long (measured from the
/// last repeatable binding).
pub const REPEAT_TIME: Duration = Duration::from_millis(500);

/// The time [`Tmux::key`] assumes between two key presses: longer than
/// [`REPEAT_TIME`], so untimed keys never repeat.
pub const UNTIMED_KEY_GAP: Duration = Duration::from_secs(1);

/// The prefix key when the tables bind no `send-prefix` (tmux's default).
const DEFAULT_PREFIX: Key = Key {
    base: KeyBase::Char('b'),
    ctrl: true,
    meta: false,
    shift: false,
};

/// A `-r` binding ran: its table stays active until `REPEAT_TIME` passes.
#[derive(Clone, Debug)]
struct Repeat {
    table: String,
    since: Duration,
}

/// A tmux server with one attached client, driven by key presses. The model
/// is the fixed 80×24 terminal (`commands::PANE_AREA` plus the status line).
#[derive(Clone, Debug)]
pub struct Tmux {
    server: Server,
    tables: KeyTables,
    prefix_key: Key,
    prefix_pending: bool,
    repeat: Option<Repeat>,
    prompt: Option<Prompt>,
    /// Time of the last key press.
    now: Duration,
}

impl Tmux {
    /// A client attached to `server`, using `tables`. The prefix key is the
    /// key bound to `send-prefix` in the `prefix` table (normally `C-b`), or
    /// `C-b` if there is none.
    pub fn new(server: Server, tables: KeyTables) -> Tmux {
        let prefix_key = tables
            .bindings("prefix")
            .find(|b| {
                b.commands
                    .first()
                    .and_then(|c| c.0.first())
                    .is_some_and(|name| name == "send-prefix")
            })
            .map_or(DEFAULT_PREFIX, |b| b.key);
        Tmux {
            server,
            tables,
            prefix_key,
            prefix_pending: false,
            repeat: None,
            prompt: None,
            now: Duration::ZERO,
        }
    }

    /// Presses `k` [`UNTIMED_KEY_GAP`] after the previous key, so it is
    /// never taken as a `-r` repeat.
    pub fn key(&mut self, k: &Key) -> KeyOutcome {
        let now = self.now.saturating_add(UNTIMED_KEY_GAP);
        self.key_at(k, now)
    }

    /// Presses `k` at time `now` (any monotonic clock). The status-line
    /// message is cleared first. An open prompt takes the key; otherwise a
    /// pending prefix looks it up in `prefix` (an unbound key just clears the
    /// prefix); otherwise, within [`REPEAT_TIME`] of a `-r` binding, a `-r`
    /// key of the same table runs without the prefix; otherwise the prefix
    /// key sets the prefix and other keys are looked up in `root`.
    pub fn key_at(&mut self, k: &Key, now: Duration) -> KeyOutcome {
        self.now = now;
        self.server.message = None;
        if let Some(prompt) = self.prompt.as_mut() {
            let text_entry = matches!(prompt, Prompt::Input { .. }) && inserts_text(k);
            match prompt.key(k) {
                PromptStep::Open => {}
                PromptStep::Closed => self.prompt = None,
                PromptStep::Run(commands) => {
                    self.prompt = None;
                    self.run_commands(&commands);
                }
                PromptStep::Failed(message) => {
                    self.prompt = None;
                    self.server.message = Some(message);
                }
            }
            return KeyOutcome {
                text_entry,
                ran: None,
            };
        }
        if self.prefix_pending {
            self.prefix_pending = false;
            let ran = self
                .tables
                .lookup("prefix", k)
                .cloned()
                .map(|b| self.run_binding(&b));
            return KeyOutcome {
                text_entry: false,
                ran,
            };
        }
        if let Some(r) = self.repeat.take() {
            let repeated = (now.saturating_sub(r.since) < REPEAT_TIME)
                .then(|| self.tables.lookup(&r.table, k))
                .flatten()
                .filter(|b| b.repeat)
                .cloned();
            if let Some(b) = repeated {
                return KeyOutcome {
                    text_entry: false,
                    ran: Some(self.run_binding(&b)),
                };
            }
            // Repeat is over: the key is handled as a fresh one.
        }
        let mut ran = None;
        if *k == self.prefix_key {
            self.prefix_pending = true;
        } else if let Some(b) = self.tables.lookup("root", k).cloned() {
            ran = Some(self.run_binding(&b));
        }
        KeyOutcome {
            text_entry: false,
            ran,
        }
    }

    /// Runs `binding` and describes it for the [`KeyOutcome`].
    fn run_binding(&mut self, binding: &Binding) -> RanBinding {
        self.run_commands(&binding.commands);
        if binding.repeat && self.prompt.is_none() {
            self.repeat = Some(Repeat {
                table: binding.table.clone(),
                since: self.now,
            });
        }
        RanBinding {
            table: binding.table.clone(),
            key: binding.key,
            command: binding
                .commands
                .first()
                .and_then(|c| c.0.first())
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// Runs `commands` in order, stopping at the first error (its message is
    /// already on the status line). A prompt or confirm they ask for opens.
    fn run_commands(&mut self, commands: &[CommandLine]) {
        for command in commands {
            match run(&mut self.server, command) {
                Ok(Effect::None) => {}
                Ok(Effect::Prompt(request)) => self.prompt = Some(Prompt::from(request)),
                Ok(Effect::Confirm { prompt, then }) => {
                    self.prompt = Some(Prompt::Confirm { prompt, then });
                }
                Err(_) => break,
            }
        }
    }

    /// The client's view (window, status line, prefix indicator, prompt or
    /// message) as an 80×24 grid: the model is a fixed 80×24 terminal
    /// (Ruling 13), and the app scales the grid rather than resizing tmux.
    pub fn grid(&self) -> CellGrid {
        let overlay = Overlay {
            prefix_pending: self.prefix_pending,
            prompt: self.prompt.as_ref().map(Prompt::status),
            message: self.server.message.clone(),
        };
        // PANE_AREA plus the status line.
        let (cols, rows) = (PANE_AREA.w, PANE_AREA.h.saturating_add(1));
        render(&self.server, &overlay, cols, rows)
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    /// The prefix key was pressed and the next key will be looked up in the
    /// `prefix` table. (Not set during a `-r` repeat window.)
    pub fn prefix_pending(&self) -> bool {
        self.prefix_pending
    }

    /// An input prompt is open: keys go to it as typed text. False while a
    /// confirm is open (its answer is a binding key).
    pub fn in_text_entry(&self) -> bool {
        matches!(self.prompt, Some(Prompt::Input { .. }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::state::{Layout, PaneId, SplitDir, Window};
    use crate::stock;
    use kf_core::key::parse_keys;

    fn tmux(fixture: &str) -> Tmux {
        Tmux::new(fixtures::get(fixture).unwrap(), stock::tables().clone())
    }

    /// Presses `keys` (space-separated) and returns each outcome.
    fn press(t: &mut Tmux, keys: &str) -> Vec<KeyOutcome> {
        parse_keys(keys).unwrap().iter().map(|k| t.key(k)).collect()
    }

    fn win(t: &Tmux) -> &Window {
        t.server()
            .attached_session()
            .unwrap()
            .current_window()
            .unwrap()
    }

    fn key(s: &str) -> Key {
        s.parse().unwrap()
    }

    #[test]
    fn prefix_percent_splits() {
        let mut t = tmux("one-pane");
        assert!(press(&mut t, "C-b").iter().all(|o| !o.text_entry));
        assert!(t.prefix_pending());
        press(&mut t, "%");
        assert!(!t.prefix_pending());
        assert_eq!(win(&t).panes.len(), 2);
        assert!(matches!(
            win(&t).layout,
            Layout::Split {
                dir: SplitDir::H,
                ..
            }
        ));
    }

    #[test]
    fn unbound_key_after_prefix_does_nothing() {
        let mut t = tmux("two-panes-h");
        let before = t.server().clone();
        press(&mut t, "C-b k");
        assert!(!t.prefix_pending());
        assert_eq!(t.server(), &before);
        // The prefix is gone: `%` alone is not a binding.
        press(&mut t, "%");
        assert_eq!(t.server(), &before);
    }

    #[test]
    fn prefix_twice_does_nothing() {
        let mut t = tmux("two-panes-h");
        let before = t.server().clone();
        press(&mut t, "C-b C-b");
        assert!(!t.prefix_pending());
        assert_eq!(t.server(), &before);
    }

    #[test]
    fn rename_prompt() {
        let mut t = tmux("one-pane");
        press(&mut t, "C-b ,");
        assert!(t.in_text_entry());
        assert!(t.grid().row_text(23).starts_with("(rename-window) bash"));
        let typed = press(&mut t, "C-u l o g s Enter");
        let entry: Vec<bool> = typed.iter().map(|o| o.text_entry).collect();
        // Ruling 17: only inserted characters are text entry.
        assert_eq!(entry, [false, true, true, true, true, false]);
        assert!(typed.iter().all(|o| o.ran.is_none()));
        assert!(!t.in_text_entry());
        assert_eq!(win(&t).name, "logs");
    }

    #[test]
    fn prompt_editing_keys_are_not_text_entry() {
        for k in ["BSpace", "C-u", "Enter", "Escape", "C-c"] {
            let mut t = tmux("one-pane");
            press(&mut t, "C-b ,");
            assert!(!t.key(&key(k)).text_entry, "{k}");
        }
        let mut t = tmux("one-pane");
        press(&mut t, "C-b ,");
        assert!(t.key(&key("Space")).text_entry);
    }

    #[test]
    fn outcome_reports_the_binding_that_ran() {
        let mut t = tmux("one-pane");
        let out = press(&mut t, "C-b %");
        assert_eq!(out[0].ran, None);
        assert_eq!(
            out[1].ran,
            Some(RanBinding {
                table: "prefix".into(),
                key: key("%"),
                command: "split-window".into(),
            })
        );
        // Unbound keys and the prefix key itself report nothing.
        assert!(press(&mut t, "C-b k a").iter().all(|o| o.ran.is_none()));
    }

    #[test]
    fn outcome_reports_root_bindings() {
        let (tables, _) =
            kf_core::bindings::parse_list_keys("bind-key -T root M-v split-window -h\n");
        let mut t = Tmux::new(fixtures::get("one-pane").unwrap(), tables);
        let ran = t.key(&key("M-v")).ran.unwrap();
        assert_eq!(
            (ran.table.as_str(), ran.command.as_str()),
            ("root", "split-window")
        );
    }

    #[test]
    fn outcome_reports_repeated_binding() {
        let mut t = tmux("four-panes");
        t.key_at(&key("C-b"), Duration::from_millis(1000));
        t.key_at(&key("Up"), Duration::from_millis(1100));
        let out = t.key_at(&key("Up"), Duration::from_millis(1300));
        let ran = out.ran.unwrap();
        assert_eq!(ran.table, "prefix");
        assert_eq!(ran.key, key("Up"));
        assert_eq!(ran.command, "select-pane");
    }

    #[test]
    fn rename_prompt_edits_prefilled_name() {
        let mut t = tmux("one-pane");
        press(&mut t, "C-b , BSpace BSpace s h 2 Space x Enter");
        assert_eq!(win(&t).name, "bash2 x");
    }

    #[test]
    fn rename_cancelled_or_empty_keeps_name() {
        for keys in ["C-b , Escape", "C-b , C-c", "C-b , C-u Enter"] {
            let mut t = tmux("one-pane");
            press(&mut t, keys);
            assert!(!t.in_text_entry(), "{keys}");
            assert_eq!(win(&t).name, "bash", "{keys}");
            assert_eq!(t.server().message, None, "{keys}");
        }
    }

    #[test]
    fn confirm_kill_pane() {
        let mut t = tmux("two-panes-h");
        press(&mut t, "C-b x");
        assert!(!t.in_text_entry());
        assert!(t.grid().row_text(23).starts_with("kill-pane 1? (y/n)"));
        assert!(!press(&mut t, "y")[0].text_entry);
        assert_eq!(win(&t).panes.len(), 1);

        let mut t = tmux("two-panes-h");
        press(&mut t, "C-b x");
        assert!(!press(&mut t, "n")[0].text_entry);
        assert!(t.grid().row_text(23).starts_with("[0] "));
        assert_eq!(win(&t).panes.len(), 2);
    }

    #[test]
    fn command_prompt_runs_typed_command() {
        let mut t = tmux("one-pane");
        assert!(press(&mut t, "C-b :").iter().all(|o| !o.text_entry));
        assert!(t.grid().row_text(23).starts_with(':'));
        let typed = press(&mut t, "s p l i t w Space - h Enter");
        assert_eq!(typed.len(), 10);
        let (enter, chars) = typed.split_last().unwrap();
        assert!(chars.iter().all(|o| o.text_entry));
        assert!(!enter.text_entry);
        assert_eq!(win(&t).panes.len(), 2);
        assert!(matches!(
            win(&t).layout,
            Layout::Split {
                dir: SplitDir::H,
                ..
            }
        ));
    }

    #[test]
    fn command_prompt_errors_become_messages() {
        let mut t = tmux("one-pane");
        press(&mut t, "C-b : f o o Enter");
        assert_eq!(t.server().message.as_deref(), Some("unknown command: foo"));
        assert!(t.grid().row_text(23).starts_with("unknown command: foo"));
        // The next key press clears the message.
        press(&mut t, "a");
        assert_eq!(t.server().message, None);

        press(&mut t, "C-b : r e n a m e w Space ' x Enter");
        assert_eq!(t.server().message.as_deref(), Some("unterminated quote"));
        assert_eq!(win(&t).name, "bash");
    }

    #[test]
    fn command_prompt_can_open_a_confirm() {
        let mut t = tmux("two-panes-h");
        press(&mut t, "C-b : k i l l p Enter");
        assert_eq!(win(&t).panes.len(), 1);
        let mut t = tmux("three-windows");
        press(&mut t, "C-b : c o n f i r m Space k i l l w Enter");
        assert!(t.grid().row_text(23).starts_with("Confirm 'killw'?"));
        press(&mut t, "y");
        assert_eq!(t.server().sessions[0].windows.len(), 2);
    }

    #[test]
    fn repeat_within_500ms() {
        let mut t = tmux("four-panes");
        assert_eq!(win(&t).active, PaneId(3));
        t.key_at(&key("C-b"), Duration::from_millis(1000));
        t.key_at(&key("Up"), Duration::from_millis(1100));
        assert_eq!(win(&t).active, PaneId(2));
        t.key_at(&key("Down"), Duration::from_millis(1300));
        assert_eq!(win(&t).active, PaneId(3));
        // Each repeat restarts the window.
        t.key_at(&key("Up"), Duration::from_millis(1700));
        assert_eq!(win(&t).active, PaneId(2));
    }

    #[test]
    fn no_repeat_after_500ms() {
        let mut t = tmux("four-panes");
        t.key_at(&key("C-b"), Duration::from_millis(1000));
        t.key_at(&key("Up"), Duration::from_millis(1100));
        assert_eq!(win(&t).active, PaneId(2));
        t.key_at(&key("Down"), Duration::from_millis(1700));
        assert_eq!(win(&t).active, PaneId(2));
        assert!(!t.prefix_pending());
    }

    #[test]
    fn key_without_time_never_repeats() {
        let mut t = tmux("four-panes");
        press(&mut t, "C-b Up Down");
        assert_eq!(win(&t).active, PaneId(2));
    }

    #[test]
    fn non_repeat_key_ends_repeat_and_is_handled_normally() {
        let mut t = tmux("four-panes");
        t.key_at(&key("C-b"), Duration::from_millis(1000));
        t.key_at(&key("Up"), Duration::from_millis(1100));
        // `%` is not `-r`: repeat ends and `%` goes to the root table (unbound).
        t.key_at(&key("%"), Duration::from_millis(1200));
        assert_eq!(win(&t).panes.len(), 4);
        // The prefix key during repeat starts a new prefix.
        t.key_at(&key("Up"), Duration::from_millis(1250));
        assert_eq!(win(&t).active, PaneId(2));
        t.key_at(&key("C-b"), Duration::from_millis(1300));
        assert!(t.prefix_pending());
    }

    #[test]
    fn grid_is_80_by_24() {
        let g = tmux("one-pane").grid();
        assert_eq!((g.cols, g.rows), (80, 24));
    }

    #[test]
    fn grid_shows_prefix_indicator() {
        let mut t = tmux("one-pane");
        assert!(!t.grid().row_text(23).contains("^B"));
        press(&mut t, "C-b");
        assert!(t.grid().row_text(23).contains("^B"));
    }

    #[test]
    fn closing_last_pane_detaches_without_panic() {
        let mut t = tmux("one-pane");
        press(&mut t, "C-b x y");
        assert_eq!(t.server().attached, None);
        press(&mut t, "C-b % C-b : s p l i t w Enter a");
        assert_eq!(t.server().attached, None);
        assert_eq!(t.grid().row_text(0).trim_end(), "[exited]");
    }

    #[test]
    fn prefix_key_comes_from_send_prefix_binding() {
        let (tables, _) = kf_core::bindings::parse_list_keys(
            "bind-key -T prefix C-a send-prefix\nbind-key -T prefix % split-window -h\n",
        );
        let mut t = Tmux::new(fixtures::get("one-pane").unwrap(), tables);
        press(&mut t, "C-b %");
        assert_eq!(win(&t).panes.len(), 1);
        press(&mut t, "C-a %");
        assert_eq!(win(&t).panes.len(), 2);
    }

    #[test]
    fn any_key_sequence_is_safe() {
        let all = "C-b : Up Down Left Right x y n , % \" c & ! z o ; Escape Enter BSpace C-u Space q 0 1 9 w s d $ C-c Tab F1 M-x";
        let keys = parse_keys(all).unwrap();
        for fixture in fixtures::NAMES {
            let mut t = tmux(fixture);
            for (i, _) in keys.iter().enumerate() {
                for k in keys.iter().skip(i).chain(keys.iter().take(i)) {
                    t.key(k);
                    let _ = t.grid();
                }
            }
        }
    }
}
