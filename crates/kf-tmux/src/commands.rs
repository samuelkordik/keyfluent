//! tmux commands that change panes and windows.

use kf_core::bindings::{parse_block, CommandLine};
use thiserror::Error;

use crate::layout::{geometry, neighbour_wrapping, Dir, Rect};
use crate::state::{Layout, Pane, PaneId, Server, Session, SplitDir, Window};

/// An open `command-prompt`: what the client shows and what runs on Enter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptRequest {
    /// Label before the input, e.g. `"(rename-window) "`.
    pub prompt: String,
    /// Starting input (`-I`), with `#W`/`#S`/`#P` already expanded.
    pub initial: String,
    /// Commands run with `%%` replaced by the input; empty = run the input
    /// itself as a command (bare `command-prompt`, prefix `:`).
    pub template: Vec<CommandLine>,
}

/// What the client must do after a command ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Prompt(PromptRequest),
    /// Ask `prompt`; on `y` run `then`.
    Confirm {
        prompt: String,
        then: Vec<CommandLine>,
    },
}

/// Why a command did not run. `run` also puts the text on the status line.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CmdError {
    #[error("unknown command: {0}")]
    Unknown(String),
    #[error("not simulated yet: {0}")]
    NotSimulated(String),
    #[error("no space for new pane")]
    NoSpace,
    #[error("{0}")]
    BadArgs(String),
}

/// Pane area of the default 80×24 terminal (one row is the status line).
/// Pane moves and the no-space check use it.
pub const PANE_AREA: Rect = Rect {
    x: 0,
    y: 0,
    w: 80,
    h: 23,
};

/// tmux's smallest pane size (`PANE_MINIMUM`); a split needs two of them
/// plus a border.
const PANE_MINIMUM: u16 = 1;

/// Name of a new window (tmux's automatic-rename shows the shell).
const DEFAULT_WINDOW_NAME: &str = "bash";

/// Content of a new pane.
const NEW_PANE_PROMPT: &str = "user@host:~$ ";

/// Every tmux 3.4 command (name, alias) not simulated here: these return
/// [`CmdError::NotSimulated`]; any other unknown name is [`CmdError::Unknown`].
const OTHER_COMMANDS: &[(&str, &str)] = &[
    ("attach-session", "attach"),
    ("bind-key", "bind"),
    ("break-pane", "breakp"),
    ("capture-pane", "capturep"),
    ("choose-buffer", ""),
    ("choose-client", ""),
    ("choose-tree", ""),
    ("clear-history", "clearhist"),
    ("clear-prompt-history", "clearphist"),
    ("clock-mode", ""),
    ("copy-mode", ""),
    ("customize-mode", ""),
    ("delete-buffer", "deleteb"),
    ("detach-client", "detach"),
    ("display-menu", "menu"),
    ("display-panes", "displayp"),
    ("display-popup", "popup"),
    ("find-window", "findw"),
    ("has-session", "has"),
    ("if-shell", "if"),
    ("join-pane", "joinp"),
    ("kill-server", ""),
    ("kill-session", ""),
    ("link-window", "linkw"),
    ("list-buffers", "lsb"),
    ("list-clients", "lsc"),
    ("list-commands", "lscm"),
    ("list-keys", "lsk"),
    ("list-panes", "lsp"),
    ("list-sessions", "ls"),
    ("list-windows", "lsw"),
    ("load-buffer", "loadb"),
    ("lock-client", "lockc"),
    ("lock-server", "lock"),
    ("lock-session", "locks"),
    ("move-pane", "movep"),
    ("move-window", "movew"),
    ("new-session", "new"),
    ("next-layout", "nextl"),
    ("paste-buffer", "pasteb"),
    ("pipe-pane", "pipep"),
    ("previous-layout", "prevl"),
    ("refresh-client", "refresh"),
    ("rename-session", "rename"),
    ("resize-window", "resizew"),
    ("respawn-pane", "respawnp"),
    ("respawn-window", "respawnw"),
    ("rotate-window", "rotatew"),
    ("run-shell", "run"),
    ("save-buffer", "saveb"),
    ("select-layout", "selectl"),
    ("send-keys", "send"),
    ("server-access", ""),
    ("set-buffer", "setb"),
    ("set-environment", "setenv"),
    ("set-hook", ""),
    ("set-option", "set"),
    ("set-window-option", "setw"),
    ("show-buffer", "showb"),
    ("show-environment", "showenv"),
    ("show-hooks", ""),
    ("show-messages", "showmsgs"),
    ("show-options", "show"),
    ("show-prompt-history", "showphist"),
    ("show-window-options", "showw"),
    ("source-file", "source"),
    ("start-server", "start"),
    ("suspend-client", "suspendc"),
    ("swap-pane", "swapp"),
    ("swap-window", "swapw"),
    ("switch-client", "switchc"),
    ("unbind-key", "unbind"),
    ("unlink-window", "unlinkw"),
    ("wait-for", "wait"),
];

/// Runs one command against the attached session. On error the error text
/// is also put on the status line (`server.message`); commands that fail
/// harmlessly in tmux (`no last pane`, `no next window`, …) set the message and
/// return `Ok(Effect::None)` with the state unchanged. Never panics.
pub fn run(server: &mut Server, cmd: &CommandLine) -> Result<Effect, CmdError> {
    let result = dispatch(server, cmd);
    if let Err(e) = &result {
        server.message = Some(e.to_string());
    }
    result
}

fn dispatch(server: &mut Server, cmd: &CommandLine) -> Result<Effect, CmdError> {
    let Some((name, args)) = cmd.0.split_first() else {
        return Err(CmdError::BadArgs("empty command".into()));
    };
    match name.as_str() {
        "split-window" | "splitw" => split_window(server, &parse_args(name, args, "c:dhv")?),
        "select-pane" | "selectp" => {
            let a = parse_args(name, args, "DLlMmRUZt:")?;
            if a.has('m') || a.has('M') {
                // Marking panes is part of P4's pane management.
                return Err(CmdError::NotSimulated("select-pane".into()));
            }
            select_pane(server, &a)
        }
        "last-pane" | "lastp" => {
            let a = parse_args(name, args, "Z")?;
            last_pane(server, a.has('Z'))
        }
        "resize-pane" | "resizep" => {
            if parse_args(name, args, "DLRUZt:x:y:")?.has('Z') {
                toggle_zoom(server)
            } else {
                Err(CmdError::NotSimulated("resize-pane".into()))
            }
        }
        "kill-pane" | "killp" => {
            parse_args(name, args, "")?;
            kill_pane(server)
        }
        "new-window" | "neww" => new_window(server, &parse_args(name, args, "c:dn:")?),
        "select-window" | "selectw" => select_window(server, &parse_args(name, args, "lnpt:")?),
        "next-window" | "next" => {
            parse_args(name, args, "a")?;
            step_window(server, true)
        }
        "previous-window" | "prev" => {
            parse_args(name, args, "a")?;
            step_window(server, false)
        }
        "last-window" | "last" => {
            parse_args(name, args, "")?;
            last_window(server)
        }
        "rename-window" | "renamew" => rename_window(server, &parse_args(name, args, "t:")?),
        "kill-window" | "killw" => {
            let a = parse_args(name, args, "t:")?;
            kill_window_cmd(server, a.value('t'))
        }
        "confirm-before" | "confirm" => confirm_before(server, &parse_args(name, args, "bp:t:y")?),
        "command-prompt" => command_prompt(server, &parse_args(name, args, "1beFiklI:Np:t:T:W")?),
        "send-prefix" => {
            parse_args(name, args, "2t:")?;
            Ok(Effect::None)
        }
        "display-message" | "display" => {
            display_message(server, &parse_args(name, args, "aCc:d:F:IlNpt:v")?)
        }
        other => {
            if OTHER_COMMANDS
                .iter()
                .any(|(n, alias)| *n == other || (!alias.is_empty() && *alias == other))
            {
                Err(CmdError::NotSimulated(other.to_owned()))
            } else {
                Err(CmdError::Unknown(other.to_owned()))
            }
        }
    }
}

/// Parsed flags of one command (tmux getopt style: flags stop at the first
/// non-flag word or `--`).
struct Args {
    flags: Vec<char>,
    values: Vec<(char, String)>,
    rest: Vec<String>,
}

impl Args {
    fn has(&self, flag: char) -> bool {
        self.flags.contains(&flag) || self.values.iter().any(|(f, _)| *f == flag)
    }

    /// The value of `-flag`; the last one wins.
    fn value(&self, flag: char) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(f, _)| *f == flag)
            .map(|(_, v)| v.as_str())
    }
}

/// Splits `args` by `spec` (getopt syntax: `x` = flag, `x:` = flag with a value).
fn parse_args(name: &str, args: &[String], spec: &str) -> Result<Args, CmdError> {
    let takes_value = |c: char| -> Option<bool> {
        let mut chars = spec.chars().peekable();
        while let Some(s) = chars.next() {
            let has_value = chars.peek() == Some(&':');
            if has_value {
                chars.next();
            }
            if s == c {
                return Some(has_value);
            }
        }
        None
    };
    let mut out = Args {
        flags: Vec::new(),
        values: Vec::new(),
        rest: Vec::new(),
    };
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        let Some(cluster) = word.strip_prefix('-').filter(|c| !c.is_empty()) else {
            out.rest.push(word.clone());
            break;
        };
        for (i, c) in cluster.char_indices() {
            match takes_value(c) {
                None => return Err(CmdError::BadArgs(format!("{name}: unknown flag -{c}"))),
                Some(false) => out.flags.push(c),
                Some(true) => {
                    let attached = cluster.get(i + c.len_utf8()..).unwrap_or_default();
                    let value = if attached.is_empty() {
                        words.next().cloned().ok_or_else(|| {
                            CmdError::BadArgs(format!("{name}: -{c} expects an argument"))
                        })?
                    } else {
                        attached.to_owned()
                    };
                    out.values.push((c, value));
                    break;
                }
            }
        }
    }
    out.rest.extend(words.cloned());
    Ok(out)
}

fn session_mut(server: &mut Server) -> Result<&mut Session, CmdError> {
    server
        .attached
        .and_then(|i| server.sessions.get_mut(i))
        .ok_or_else(|| CmdError::BadArgs("no current session".into()))
}

fn current_window_mut(server: &mut Server) -> Result<&mut Window, CmdError> {
    let session = session_mut(server)?;
    let current = session.current;
    session
        .windows
        .iter_mut()
        .find(|w| w.index == current)
        .ok_or_else(|| CmdError::BadArgs("no current window".into()))
}

/// Pane ids in tmux pane-index order.
fn pane_order(window: &Window) -> Vec<PaneId> {
    geometry(&window.layout, PANE_AREA)
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

fn pane_index(window: &Window, id: PaneId) -> Option<usize> {
    pane_order(window).iter().position(|p| *p == id)
}

/// Expands `#S #I #W #P ##` and their `#{…}` long forms; other text is kept.
fn expand(text: &str, server: &Server) -> String {
    let session = server.attached_session();
    let window = session.and_then(Session::current_window);
    let value = |key: &str| -> Option<String> {
        match key {
            "S" | "session_name" => session.map(|s| s.name.clone()),
            "I" | "window_index" => window.map(|w| w.index.to_string()),
            "W" | "window_name" => window.map(|w| w.name.clone()),
            "P" | "pane_index" => window
                .and_then(|w| pane_index(w, w.active))
                .map(|i| i.to_string()),
            _ => None,
        }
    };
    let mut out = String::new();
    let mut rest = text;
    while let Some(pos) = rest.find('#') {
        out.push_str(rest.get(..pos).unwrap_or_default());
        let after = rest.get(pos + 1..).unwrap_or_default();
        let (replacement, consumed) = if let Some(long) = after.strip_prefix('{') {
            match long.find('}') {
                Some(end) => (
                    value(long.get(..end).unwrap_or_default()),
                    end + 2, // `{` … `}`
                ),
                None => (None, 0),
            }
        } else if after.starts_with('#') {
            (Some("#".to_owned()), 1)
        } else {
            match after.chars().next() {
                Some(c) => (value(&c.to_string()), c.len_utf8()),
                None => (None, 0),
            }
        };
        match replacement {
            Some(r) => {
                out.push_str(&r);
                rest = after.get(consumed..).unwrap_or_default();
            }
            None => {
                out.push('#');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A `{ … }` block argument read as commands, or the words as one command.
fn command_list(words: &[String]) -> Result<Vec<CommandLine>, CmdError> {
    match words {
        [] => Ok(Vec::new()),
        [block] if block.starts_with('{') => parse_block(block)
            .ok_or_else(|| CmdError::BadArgs(format!("bad command block: {block}"))),
        _ => Ok(vec![CommandLine(words.to_vec())]),
    }
}

// --- panes ---------------------------------------------------------------

/// Replaces the `Pane(target)` leaf with `new`; false if absent.
fn replace_pane(layout: &mut Layout, target: PaneId, new: &mut Option<Layout>) -> bool {
    match layout {
        Layout::Pane(id) if *id == target => {
            if let Some(n) = new.take() {
                *layout = n;
            }
            true
        }
        Layout::Pane(_) => false,
        Layout::Split { children, .. } => children.iter_mut().any(|c| replace_pane(c, target, new)),
    }
}

/// Removes the `Pane(target)` leaf from a split, collapsing a split left
/// with one child into that child. False if absent (or `layout` is the leaf).
fn remove_pane(layout: &mut Layout, target: PaneId) -> bool {
    let Layout::Split { children, .. } = layout else {
        return false;
    };
    let removed = match children.iter().position(|c| *c == Layout::Pane(target)) {
        Some(pos) => {
            children.remove(pos);
            true
        }
        None => children.iter_mut().any(|c| remove_pane(c, target)),
    };
    if removed && children.len() == 1 {
        if let Some(only) = children.pop() {
            *layout = only;
        }
    }
    removed
}

fn split_window(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let dir = if args.has('h') {
        SplitDir::H
    } else {
        SplitDir::V
    };
    let new_id = PaneId(server.next_pane);
    let window = current_window_mut(server)?;
    let target = window.active;
    let rect = geometry(&window.layout, PANE_AREA)
        .into_iter()
        .find(|(id, _)| *id == target)
        .map(|(_, r)| r)
        .ok_or_else(|| CmdError::BadArgs("no current pane".into()))?;
    let size = match dir {
        SplitDir::H => rect.w,
        SplitDir::V => rect.h,
    };
    if size < PANE_MINIMUM * 2 + 1 {
        return Err(CmdError::NoSpace);
    }
    let mut new = Some(Layout::Split {
        dir,
        children: vec![Layout::Pane(target), Layout::Pane(new_id)],
    });
    if !replace_pane(&mut window.layout, target, &mut new) {
        return Err(CmdError::BadArgs("no current pane".into()));
    }
    window.zoomed = false;
    window.panes.push(Pane {
        id: new_id,
        lines: vec![NEW_PANE_PROMPT.to_owned()],
    });
    if !args.has('d') {
        window.last_active = Some(target);
        window.active = new_id;
    }
    server.next_pane = server.next_pane.saturating_add(1);
    Ok(Effect::None)
}

/// Makes `to` the active pane, remembering the old one; unzooms unless
/// `keep_zoom`. Selecting the active pane changes nothing.
fn set_active_pane(window: &mut Window, to: PaneId, keep_zoom: bool) {
    if to == window.active {
        return;
    }
    window.last_active = Some(window.active);
    window.active = to;
    if !keep_zoom {
        window.zoomed = false;
    }
}

fn select_pane(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    if args.has('l') {
        return last_pane(server, args.has('Z'));
    }
    let keep_zoom = args.has('Z');
    let window = current_window_mut(server)?;
    let dir = [
        ('L', Dir::Left),
        ('R', Dir::Right),
        ('U', Dir::Up),
        ('D', Dir::Down),
    ]
    .into_iter()
    .find(|(f, _)| args.has(*f))
    .map(|(_, d)| d);
    let to = if let Some(dir) = dir {
        neighbour_wrapping(&window.layout, PANE_AREA, window.active, dir)
    } else if let Some(target) = args.value('t') {
        Some(resolve_pane(window, target)?)
    } else {
        None
    };
    if let Some(to) = to {
        set_active_pane(window, to, keep_zoom);
    }
    Ok(Effect::None)
}

/// A pane target: `N`, `.N`, `:.N`, `:.+`, `:.-` (relative to the active
/// pane, wrapping) or `%ID`, within the current window.
fn resolve_pane(window: &Window, target: &str) -> Result<PaneId, CmdError> {
    let not_found = || CmdError::BadArgs(format!("can't find pane: {target}"));
    let order = pane_order(window);
    if let Some(id) = target.strip_prefix('%') {
        let id = PaneId(id.parse().map_err(|_| not_found())?);
        return order.contains(&id).then_some(id).ok_or_else(not_found);
    }
    let pane = match target.rsplit_once('.') {
        Some(("" | ":", pane)) => pane,
        Some(_) => return Err(not_found()),
        None => target,
    };
    let count = order.len();
    let active = order
        .iter()
        .position(|p| *p == window.active)
        .ok_or_else(not_found)?;
    let index = match pane {
        "+" => (active + 1) % count.max(1),
        "-" => (active + count.saturating_sub(1)) % count.max(1),
        n => n.parse().map_err(|_| not_found())?,
    };
    order.get(index).copied().ok_or_else(not_found)
}

fn last_pane(server: &mut Server, keep_zoom: bool) -> Result<Effect, CmdError> {
    let window = current_window_mut(server)?;
    match window.last_active.filter(|id| window.pane(*id).is_some()) {
        Some(last) => set_active_pane(window, last, keep_zoom),
        None => server.message = Some("no last pane".into()),
    }
    Ok(Effect::None)
}

fn toggle_zoom(server: &mut Server) -> Result<Effect, CmdError> {
    let window = current_window_mut(server)?;
    // tmux cannot zoom a window with one pane.
    if window.zoomed || window.panes.len() > 1 {
        window.zoomed = !window.zoomed;
    }
    Ok(Effect::None)
}

fn kill_pane(server: &mut Server) -> Result<Effect, CmdError> {
    let window = current_window_mut(server)?;
    let target = window.active;
    if window.panes.len() <= 1 {
        let index = window.index;
        return kill_window(server, index);
    }
    let order = pane_order(window);
    let pos = order.iter().position(|p| *p == target);
    if !remove_pane(&mut window.layout, target) {
        return Err(CmdError::BadArgs("no current pane".into()));
    }
    window.panes.retain(|p| p.id != target);
    window.zoomed = false;
    // tmux: the last pane, else the previous by index, else the next.
    let previous = pos
        .and_then(|p| p.checked_sub(1))
        .and_then(|p| order.get(p));
    let next = pos.and_then(|p| order.get(p + 1));
    let replacement = window
        .last_active
        .filter(|id| *id != target && window.pane(*id).is_some())
        .or(previous.copied())
        .or(next.copied());
    if let Some(r) = replacement {
        window.active = r;
    }
    window.last_active = None;
    Ok(Effect::None)
}

// --- windows -------------------------------------------------------------

/// Makes window `index` current, remembering the old one. Selecting the
/// current window changes nothing.
fn set_current_window(session: &mut Session, index: u32) {
    if index != session.current {
        session.last = Some(session.current);
        session.current = index;
    }
}

fn new_window(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let pane_id = PaneId(server.next_pane);
    let session = session_mut(server)?;
    let index = (0..=u32::MAX)
        .find(|i| session.window(*i).is_none())
        .ok_or_else(|| CmdError::BadArgs("no free window index".into()))?;
    session.windows.push(Window {
        index,
        name: args.value('n').unwrap_or(DEFAULT_WINDOW_NAME).to_owned(),
        layout: Layout::Pane(pane_id),
        panes: vec![Pane {
            id: pane_id,
            lines: vec![NEW_PANE_PROMPT.to_owned()],
        }],
        active: pane_id,
        last_active: None,
        zoomed: false,
    });
    session.windows.sort_by_key(|w| w.index);
    if !args.has('d') {
        set_current_window(session, index);
    }
    server.next_pane = server.next_pane.saturating_add(1);
    Ok(Effect::None)
}

/// A window target: `N`, `:N`, `:=N`, `=N` or a window name; `:` alone is
/// the current window.
fn resolve_window(session: &Session, target: &str) -> Result<u32, CmdError> {
    let bare = target.strip_prefix(':').unwrap_or(target);
    let bare = bare.strip_prefix('=').unwrap_or(bare);
    if bare.is_empty() {
        return Ok(session.current);
    }
    let found = match bare.parse::<u32>() {
        Ok(i) => session.window(i).map(|w| w.index),
        Err(_) => session
            .windows
            .iter()
            .find(|w| w.name == bare)
            .map(|w| w.index),
    };
    found.ok_or_else(|| CmdError::BadArgs(format!("can't find window: {bare}")))
}

fn select_window(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    if args.has('l') {
        return last_window(server);
    }
    if args.has('n') || args.has('p') {
        return step_window(server, args.has('n'));
    }
    let session = session_mut(server)?;
    let index = match args.value('t') {
        Some(t) => resolve_window(session, t)?,
        None => session.current,
    };
    set_current_window(session, index);
    Ok(Effect::None)
}

/// `next-window` / `previous-window`: by index, wrapping.
fn step_window(server: &mut Server, forward: bool) -> Result<Effect, CmdError> {
    let session = session_mut(server)?;
    let mut indices: Vec<u32> = session.windows.iter().map(|w| w.index).collect();
    indices.sort_unstable();
    if indices.len() < 2 {
        let which = if forward { "next" } else { "previous" };
        server.message = Some(format!("no {which} window"));
        return Ok(Effect::None);
    }
    let current = session.current;
    let target = if forward {
        indices.iter().find(|i| **i > current).or(indices.first())
    } else {
        indices
            .iter()
            .rev()
            .find(|i| **i < current)
            .or(indices.last())
    };
    if let Some(&t) = target {
        set_current_window(session, t);
    }
    Ok(Effect::None)
}

fn last_window(server: &mut Server) -> Result<Effect, CmdError> {
    let session = session_mut(server)?;
    match session.last.filter(|l| session.window(*l).is_some()) {
        Some(last) => set_current_window(session, last),
        None => server.message = Some("no last window".into()),
    }
    Ok(Effect::None)
}

fn rename_window(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let [name] = args.rest.as_slice() else {
        return Err(CmdError::BadArgs("usage: rename-window NAME".into()));
    };
    let session = session_mut(server)?;
    let index = match args.value('t') {
        Some(t) => resolve_window(session, t)?,
        None => session.current,
    };
    if let Some(w) = session.windows.iter_mut().find(|w| w.index == index) {
        w.name.clone_from(name);
    }
    Ok(Effect::None)
}

fn kill_window_cmd(server: &mut Server, target: Option<&str>) -> Result<Effect, CmdError> {
    let session = session_mut(server)?;
    let index = match target {
        Some(t) => resolve_window(session, t)?,
        None => session.current,
    };
    kill_window(server, index)
}

/// Removes window `index` of the attached session. If it was current, the
/// last window takes over, else the previous by index, else the next. The
/// session ends with its last window and the client detaches (tmux's
/// default `detach-on-destroy on`).
fn kill_window(server: &mut Server, index: u32) -> Result<Effect, CmdError> {
    let attached = server.attached;
    let session = session_mut(server)?;
    session.windows.retain(|w| w.index != index);
    if session.windows.is_empty() {
        if let Some(i) = attached {
            if i < server.sessions.len() {
                server.sessions.remove(i);
            }
        }
        server.attached = None;
        return Ok(Effect::None);
    }
    if session.current == index {
        let last = session.last.filter(|l| session.window(*l).is_some());
        let previous = session.windows.iter().rev().find(|w| w.index < index);
        let next = session.windows.iter().find(|w| w.index > index);
        if let Some(new) = last.or(previous.or(next).map(|w| w.index)) {
            session.current = new;
        }
        session.last = None;
    } else if session.last == Some(index) {
        session.last = None;
    }
    Ok(Effect::None)
}

// --- prompts and messages ------------------------------------------------

fn confirm_before(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let then = command_list(&args.rest)?;
    let Some(first) = then.first().and_then(|c| c.0.first()) else {
        return Err(CmdError::BadArgs("usage: confirm-before COMMAND".into()));
    };
    let prompt = match args.value('p') {
        Some(p) => format!("{} ", expand(p, server)),
        None => format!("Confirm '{first}'? (y/n) "),
    };
    Ok(Effect::Confirm { prompt, then })
}

fn command_prompt(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let template = command_list(&args.rest)?;
    let prompt = match (args.value('p'), template.first().and_then(|c| c.0.first())) {
        (Some(p), _) => format!("{} ", expand(p, server)),
        (None, Some(name)) => format!("({name}) "),
        (None, None) => ":".to_owned(),
    };
    let initial = args
        .value('I')
        .map(|i| expand(i, server))
        .unwrap_or_default();
    Ok(Effect::Prompt(PromptRequest {
        prompt,
        initial,
        template,
    }))
}

/// tmux 3.4's default `display-message` text without the clock (the model
/// has no time, and a fixed message keeps the grid deterministic).
const DISPLAY_MESSAGE_DEFAULT: &str = "[#S] #I:#W, current pane #P";

fn display_message(server: &mut Server, args: &Args) -> Result<Effect, CmdError> {
    let joined = args.rest.join(" ");
    let format = match args.value('F') {
        Some(f) => f,
        None if joined.is_empty() => DISPLAY_MESSAGE_DEFAULT,
        None => &joined,
    };
    server.message = Some(expand(format, server));
    Ok(Effect::None)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::stock;

    fn cmd(text: &str) -> CommandLine {
        CommandLine(text.split_whitespace().map(str::to_owned).collect())
    }

    fn fx(name: &str) -> Server {
        fixtures::get(name).unwrap()
    }

    fn ok(s: &mut Server, text: &str) -> Effect {
        run(s, &cmd(text)).unwrap()
    }

    fn win(s: &Server) -> &Window {
        s.attached_session().unwrap().current_window().unwrap()
    }

    fn indices(s: &Server) -> Vec<u32> {
        s.sessions[0].windows.iter().map(|w| w.index).collect()
    }

    #[test]
    fn split_h_on_one_pane() {
        let mut s = fx("one-pane");
        assert_eq!(ok(&mut s, "splitw -h"), Effect::None);
        let w = win(&s);
        assert_eq!(w.panes.len(), 2);
        assert_eq!(
            w.layout,
            Layout::Split {
                dir: SplitDir::H,
                children: vec![Layout::Pane(PaneId(0)), Layout::Pane(PaneId(1))]
            }
        );
        assert_eq!(w.active, PaneId(1));
        assert_eq!(w.last_active, Some(PaneId(0)));
        assert_eq!(s.next_pane, 2);
    }

    #[test]
    fn split_nests_in_place_of_the_target() {
        let mut s = fx("two-panes-h");
        ok(&mut s, "split-window -v");
        assert_eq!(
            win(&s).layout,
            Layout::Split {
                dir: SplitDir::H,
                children: vec![
                    Layout::Pane(PaneId(0)),
                    Layout::Split {
                        dir: SplitDir::V,
                        children: vec![Layout::Pane(PaneId(1)), Layout::Pane(PaneId(2))]
                    }
                ]
            }
        );
        assert_eq!(win(&s).active, PaneId(2));
    }

    #[test]
    fn ten_vertical_splits_run_out_of_space() {
        let mut s = fx("one-pane");
        let mut results = Vec::new();
        for _ in 0..10 {
            results.push(run(&mut s, &cmd("splitw -v")));
        }
        assert_eq!(results.last(), Some(&Err(CmdError::NoSpace)));
        assert_eq!(s.message.as_deref(), Some("no space for new pane"));
        // 23 rows → 11|11 → 5|5 → 2|2 → no room for a third split.
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 3);
        assert_eq!(win(&s).panes.len(), 4);
    }

    #[test]
    fn select_pane_left_and_wrap() {
        let mut s = fx("two-panes-h");
        ok(&mut s, "selectp -L");
        assert_eq!(win(&s).active, PaneId(0));
        assert_eq!(win(&s).last_active, Some(PaneId(1)));
        let mut s = fx("two-panes-h");
        ok(&mut s, "select-pane -R");
        assert_eq!(win(&s).active, PaneId(0));
        let mut s = fx("two-panes-v-first");
        ok(&mut s, "selectp -U");
        assert_eq!(win(&s).active, PaneId(1));
    }

    #[test]
    fn select_pane_targets_and_last() {
        let mut s = fx("four-panes");
        ok(&mut s, "selectp -t 1");
        assert_eq!(win(&s).active, PaneId(1));
        ok(&mut s, "select-pane -t :.+");
        assert_eq!(win(&s).active, PaneId(2));
        ok(&mut s, "select-pane -t :.-");
        assert_eq!(win(&s).active, PaneId(1));
        ok(&mut s, "lastp");
        assert_eq!(win(&s).active, PaneId(2));
        ok(&mut s, "select-pane -l");
        assert_eq!(win(&s).active, PaneId(1));
        // `o` cycles through every pane and wraps.
        let mut s = fx("four-panes");
        ok(&mut s, "select-pane -t :.+");
        assert_eq!(win(&s).active, PaneId(0));
        assert!(matches!(
            run(&mut s, &cmd("selectp -t 7")),
            Err(CmdError::BadArgs(_))
        ));
        let mut s = fx("one-pane");
        ok(&mut s, "last-pane");
        assert_eq!(s.message.as_deref(), Some("no last pane"));
    }

    #[test]
    fn resize_pane_zoom_toggles() {
        let mut s = fx("two-panes-h");
        ok(&mut s, "resizep -Z");
        assert!(win(&s).zoomed);
        ok(&mut s, "resize-pane -Z");
        assert!(!win(&s).zoomed);
        // A pane move unzooms, as in tmux.
        ok(&mut s, "resizep -Z");
        ok(&mut s, "selectp -L");
        assert!(!win(&s).zoomed);
        // A single pane cannot zoom.
        let mut s = fx("one-pane");
        ok(&mut s, "resizep -Z");
        assert!(!win(&s).zoomed);
        assert_eq!(
            run(&mut s, &cmd("resize-pane -U 5")),
            Err(CmdError::NotSimulated("resize-pane".into()))
        );
    }

    #[test]
    fn kill_pane() {
        let mut s = fx("two-panes-h");
        ok(&mut s, "killp");
        let w = win(&s);
        assert_eq!(w.panes.len(), 1);
        assert_eq!(w.layout, Layout::Pane(PaneId(0)));
        assert_eq!(w.active, PaneId(0));
        assert_eq!(w.last_active, None);
        // Without a last pane the previous pane by index takes over.
        let mut s = fx("four-panes");
        ok(&mut s, "selectp -t 1");
        ok(&mut s, "selectp -t 2");
        ok(&mut s, "selectp -t 1");
        // active p1, last p2: killing p1 goes back to p2.
        ok(&mut s, "kill-pane");
        assert_eq!(win(&s).active, PaneId(2));
        ok(&mut s, "kill-pane");
        assert_eq!(win(&s).active, PaneId(0));
        assert_eq!(
            win(&s).layout,
            Layout::Split {
                dir: SplitDir::H,
                children: vec![Layout::Pane(PaneId(0)), Layout::Pane(PaneId(3))]
            }
        );
    }

    #[test]
    fn kill_only_pane_ends_the_session() {
        let mut s = fx("one-pane");
        assert_eq!(ok(&mut s, "killp"), Effect::None);
        assert_eq!(s.attached, None);
        assert!(s.sessions.is_empty());
        // Commands on a detached server fail cleanly.
        assert!(matches!(
            run(&mut s, &cmd("killp")),
            Err(CmdError::BadArgs(_))
        ));
    }

    #[test]
    fn new_window() {
        let mut s = fx("three-windows");
        ok(&mut s, "neww");
        let session = &s.sessions[0];
        assert_eq!((session.current, session.last), (3, Some(1)));
        assert_eq!(win(&s).name, "bash");
        assert_eq!(win(&s).active, PaneId(3));
        assert_eq!(s.next_pane, 4);
        // The lowest free index is reused.
        ok(&mut s, "selectw -t 1");
        ok(&mut s, "killw");
        ok(&mut s, "new-window -n logs2");
        assert_eq!(s.sessions[0].current, 1);
        assert_eq!(win(&s).name, "logs2");
        assert_eq!(indices(&s), vec![0, 1, 2, 3]);
    }

    #[test]
    fn select_window() {
        let mut s = fx("three-windows");
        ok(&mut s, "selectw -t 2");
        assert_eq!((s.sessions[0].current, s.sessions[0].last), (2, Some(1)));
        ok(&mut s, "select-window -t :=0");
        assert_eq!((s.sessions[0].current, s.sessions[0].last), (0, Some(2)));
        ok(&mut s, "select-window -t :1");
        assert_eq!(s.sessions[0].current, 1);
        ok(&mut s, "select-window -t editor");
        assert_eq!(s.sessions[0].current, 2);
        assert!(matches!(
            run(&mut s, &cmd("select-window -t :=5")),
            Err(CmdError::BadArgs(_))
        ));
        assert_eq!(s.message.as_deref(), Some("can't find window: 5"));
        assert_eq!(s.sessions[0].current, 2);
    }

    #[test]
    fn next_and_previous_wrap() {
        let mut s = fx("three-windows");
        ok(&mut s, "next");
        assert_eq!(s.sessions[0].current, 2);
        ok(&mut s, "next-window");
        assert_eq!(s.sessions[0].current, 0);
        assert_eq!(s.sessions[0].last, Some(2));
        ok(&mut s, "prev");
        assert_eq!(s.sessions[0].current, 2);
        ok(&mut s, "previous-window");
        assert_eq!(s.sessions[0].current, 1);
        let mut s = fx("one-pane");
        ok(&mut s, "next");
        assert_eq!(s.message.as_deref(), Some("no next window"));
    }

    #[test]
    fn last_window_swaps() {
        let mut s = fx("three-windows");
        ok(&mut s, "last");
        assert_eq!((s.sessions[0].current, s.sessions[0].last), (0, Some(1)));
        ok(&mut s, "last-window");
        assert_eq!((s.sessions[0].current, s.sessions[0].last), (1, Some(0)));
        let mut s = fx("one-pane");
        ok(&mut s, "last");
        assert_eq!(s.message.as_deref(), Some("no last window"));
    }

    #[test]
    fn rename_window() {
        let mut s = fx("three-windows");
        ok(&mut s, "renamew logs2");
        assert_eq!(win(&s).name, "logs2");
        ok(&mut s, "rename-window -- spaced");
        assert_eq!(win(&s).name, "spaced");
        assert!(matches!(
            run(&mut s, &cmd("rename-window")),
            Err(CmdError::BadArgs(_))
        ));
    }

    #[test]
    fn kill_window() {
        let mut s = fx("three-windows");
        ok(&mut s, "killw");
        assert_eq!(indices(&s), vec![0, 2]);
        assert_eq!((s.sessions[0].current, s.sessions[0].last), (0, None));
        // Without a last window, the previous by index, else the next.
        ok(&mut s, "selectw -t 2");
        ok(&mut s, "selectw -t 0");
        ok(&mut s, "killw");
        assert_eq!(s.sessions[0].current, 2);
        ok(&mut s, "kill-window");
        assert_eq!(s.attached, None);
        let mut s = fx("three-windows");
        ok(&mut s, "selectw -t 0");
        ok(&mut s, "last-window");
        ok(&mut s, "last-window");
        // current 0, last 1: no last after it is killed, previous is none, next is 1.
        ok(&mut s, "killw -t :=1");
        ok(&mut s, "killw");
        assert_eq!(s.sessions[0].current, 2);
    }

    #[test]
    fn confirm_before() {
        let mut s = fx("two-panes-h");
        let x = CommandLine(vec![
            "confirm-before".into(),
            "-p".into(),
            "kill-pane #P? (y/n)".into(),
            "kill-pane".into(),
        ]);
        assert_eq!(
            run(&mut s, &x).unwrap(),
            Effect::Confirm {
                prompt: "kill-pane 1? (y/n) ".into(),
                then: vec![cmd("kill-pane")],
            }
        );
        assert_eq!(win(&s).panes.len(), 2);
        assert_eq!(
            ok(&mut s, "confirm kill-window"),
            Effect::Confirm {
                prompt: "Confirm 'kill-window'? (y/n) ".into(),
                then: vec![cmd("kill-window")],
            }
        );
    }

    #[test]
    fn command_prompt() {
        let mut s = fx("three-windows");
        let comma = CommandLine(vec![
            "command-prompt".into(),
            "-I".into(),
            "#W".into(),
            "{ rename-window \"%%\" }".into(),
        ]);
        assert_eq!(
            run(&mut s, &comma).unwrap(),
            Effect::Prompt(PromptRequest {
                prompt: "(rename-window) ".into(),
                initial: "logs".into(),
                template: vec![cmd("rename-window %%")],
            })
        );
        assert_eq!(
            ok(&mut s, "command-prompt"),
            Effect::Prompt(PromptRequest {
                prompt: ":".into(),
                initial: String::new(),
                template: vec![],
            })
        );
        assert_eq!(
            ok(&mut s, "command-prompt -p index select-window -t :%%"),
            Effect::Prompt(PromptRequest {
                prompt: "index ".into(),
                initial: String::new(),
                template: vec![cmd("select-window -t :%%")],
            })
        );
    }

    #[test]
    fn send_prefix_and_display_message() {
        let mut s = fx("three-windows");
        assert_eq!(ok(&mut s, "send-prefix"), Effect::None);
        assert_eq!(s.message, None);
        ok(&mut s, "display-message hi_#S:#I:#W.#P_##");
        assert_eq!(s.message.as_deref(), Some("hi_0:1:logs.0_#"));
        ok(&mut s, "display");
        assert_eq!(s.message.as_deref(), Some("[0] 1:logs, current pane 0"));
    }

    #[test]
    fn not_simulated_and_unknown() {
        let mut s = fx("one-pane");
        assert_eq!(
            run(&mut s, &cmd("choose-tree -Zw")),
            Err(CmdError::NotSimulated("choose-tree".into()))
        );
        assert_eq!(s.message.as_deref(), Some("not simulated yet: choose-tree"));
        assert_eq!(
            run(&mut s, &cmd("frobnicate")),
            Err(CmdError::Unknown("frobnicate".into()))
        );
        assert_eq!(s.message.as_deref(), Some("unknown command: frobnicate"));
        assert!(matches!(
            run(&mut s, &CommandLine(vec![])),
            Err(CmdError::BadArgs(_))
        ));
        assert!(matches!(
            run(&mut s, &cmd("splitw -q")),
            Err(CmdError::BadArgs(_))
        ));
        assert!(matches!(
            run(&mut s, &cmd("neww -n")),
            Err(CmdError::BadArgs(_))
        ));
    }

    /// Every stock key binding — and every command its prompt or confirm
    /// would run — is either simulated or reported as not simulated; none is
    /// unknown or has arguments the model rejects.
    #[test]
    fn every_stock_binding_runs() {
        let tables = stock::tables();
        for table in tables.tables() {
            for fixture in fixtures::NAMES {
                let mut queue: Vec<CommandLine> = Vec::new();
                for binding in tables.bindings(table) {
                    queue.extend(binding.commands.iter().cloned());
                }
                let mut s = fx(fixture);
                while let Some(c) = queue.pop() {
                    if s.attached.is_none() {
                        s = fx(fixture);
                    }
                    match run(&mut s, &c) {
                        Ok(Effect::Confirm { then, .. }) => queue.extend(then),
                        Ok(Effect::Prompt(p)) => queue.extend(p.template.into_iter().map(|t| {
                            CommandLine(t.0.into_iter().map(|a| a.replace("%%", "1")).collect())
                        })),
                        Ok(Effect::None) | Err(CmdError::NotSimulated(_) | CmdError::NoSpace) => {}
                        // A missing target (`select-window -t :=9`) fails as in tmux.
                        Err(CmdError::BadArgs(m)) if m.starts_with("can't find") => {}
                        Err(e) => panic!("{table} {fixture} {c:?}: {e:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn stock_basics_do_what_they_say() {
        let tables = stock::tables();
        let press = |s: &mut Server, key: &str| -> Effect {
            let b = tables.lookup("prefix", &key.parse().unwrap()).unwrap();
            let mut last = Effect::None;
            for c in &b.commands {
                last = run(s, c).unwrap();
            }
            last
        };
        let mut s = fx("one-pane");
        press(&mut s, "%");
        press(&mut s, "\"");
        assert_eq!(win(&s).panes.len(), 3);
        press(&mut s, "Left");
        assert_eq!(win(&s).active, PaneId(0));
        press(&mut s, "o");
        assert_eq!(win(&s).active, PaneId(1));
        press(&mut s, ";");
        assert_eq!(win(&s).active, PaneId(0));
        press(&mut s, "z");
        assert!(win(&s).zoomed);
        let Effect::Confirm { prompt, then } = press(&mut s, "x") else {
            panic!("x should confirm");
        };
        assert_eq!(prompt, "kill-pane 0? (y/n) ");
        run(&mut s, &then[0]).unwrap();
        assert_eq!(win(&s).panes.len(), 2);
        press(&mut s, "c");
        press(&mut s, "1");
        assert_eq!(s.sessions[0].current, 1);
        press(&mut s, "l");
        assert_eq!(s.sessions[0].current, 0);
        press(&mut s, "n");
        press(&mut s, "p");
        assert_eq!(s.sessions[0].current, 0);
        let Effect::Prompt(p) = press(&mut s, ",") else {
            panic!(", should prompt");
        };
        assert_eq!(p.initial, "bash");
        let Effect::Confirm { prompt, .. } = press(&mut s, "&") else {
            panic!("& should confirm");
        };
        assert_eq!(prompt, "kill-window bash? (y/n) ");
        let block = parse_block("{ kill-window }").unwrap();
        run(&mut s, &block[0]).unwrap();
        assert_eq!(indices(&s), vec![1]);
    }
}
