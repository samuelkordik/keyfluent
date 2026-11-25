//! Key tables parsed from `tmux list-keys` output.
//!
//! Tokenizer rules (tmux `cmd-parse.y`, as `list-keys` prints them):
//! - whitespace separates words; quoted and unquoted parts of one word join;
//! - `'…'` is literal; inside `"…"` and in bare words `\x` stands for `x`;
//! - a standalone unquoted `\;` or `;` separates commands, while a quoted
//!   `";"` is an ordinary argument;
//! - a `{ … }` block is kept as one argument, its source text verbatim
//!   (see [`parse_block`] to read it as a command list).

use std::collections::BTreeMap;

use thiserror::Error;

use crate::key::{Key, KeyParseError};

/// One tokenised command: its name followed by its arguments.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommandLine(pub Vec<String>);

/// A key bound in a table to a `;`-separated sequence of commands.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Binding {
    pub table: String,
    pub key: Key,
    /// Bound with `-r`: the key may repeat without the prefix.
    pub repeat: bool,
    pub commands: Vec<CommandLine>,
}

/// Key bindings by table name, then key.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct KeyTables {
    tables: BTreeMap<String, BTreeMap<Key, Binding>>,
    skipped_mouse: usize,
}

/// Why a `list-keys` line was ignored.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum ParseWarningKind {
    #[error("not a bind-key line")]
    NotBindKey,
    #[error("-T without a table name")]
    MissingTable,
    #[error("no key")]
    MissingKey,
    #[error("bad key: {0}")]
    BadKey(KeyParseError),
    #[error("no command")]
    MissingCommand,
    #[error("empty command in a sequence")]
    EmptyCommand,
    #[error("unterminated quote")]
    UnterminatedQuote,
    #[error("unterminated {{ }} block")]
    UnterminatedBlock,
}

/// A `list-keys` line that was skipped, with its 1-based line number.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
#[error("line {line}: {kind}")]
pub struct ParseWarning {
    pub line: usize,
    pub kind: ParseWarningKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Token {
    Word(String),
    /// A standalone `\;` or `;`.
    Separator,
    /// A `{ … }` block, source text including the braces.
    Block(String),
}

impl Token {
    /// The token as an argument (a separator reads as `;`).
    fn into_text(self) -> String {
        match self {
            Token::Word(s) | Token::Block(s) => s,
            Token::Separator => ";".to_owned(),
        }
    }
}

/// Split one line into tokens.
fn tokenize(line: &str) -> Result<Vec<Token>, ParseWarningKind> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let rest = line.get(start..).unwrap_or_default();
        let opens_block = rest
            .strip_prefix('{')
            .is_some_and(|t| t.chars().next().is_none_or(char::is_whitespace));
        if opens_block {
            let len = block_len(rest)?;
            tokens.push(Token::Block(rest.get(..len).unwrap_or(rest).to_owned()));
            while chars.next_if(|&(i, _)| i < start + len).is_some() {}
            continue;
        }
        let mut word = String::new();
        let mut bare = String::new();
        let mut quoted = false;
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                break;
            }
            chars.next();
            match c {
                '\\' => {
                    bare.push(c);
                    if let Some((_, n)) = chars.next() {
                        bare.push(n);
                        word.push(n);
                    } else {
                        word.push(c);
                    }
                }
                '\'' => {
                    quoted = true;
                    loop {
                        match chars.next() {
                            Some((_, '\'')) => break,
                            Some((_, n)) => word.push(n),
                            None => return Err(ParseWarningKind::UnterminatedQuote),
                        }
                    }
                }
                '"' => {
                    quoted = true;
                    loop {
                        match chars.next() {
                            Some((_, '"')) => break,
                            Some((_, '\\')) => match chars.next() {
                                Some((_, n)) => word.push(n),
                                None => return Err(ParseWarningKind::UnterminatedQuote),
                            },
                            Some((_, n)) => word.push(n),
                            None => return Err(ParseWarningKind::UnterminatedQuote),
                        }
                    }
                }
                _ => {
                    bare.push(c);
                    word.push(c);
                }
            }
        }
        if !quoted && (bare == "\\;" || bare == ";") {
            tokens.push(Token::Separator);
        } else {
            tokens.push(Token::Word(word));
        }
    }
    Ok(tokens)
}

/// Byte length of the `{ … }` block at the start of `s`, matching nested
/// braces and skipping quoted text and `\` escapes.
fn block_len(s: &str) -> Result<usize, ParseWarningKind> {
    let mut depth = 0usize;
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(i + 1);
                }
            }
            '\\' => {
                chars.next();
            }
            '\'' | '"' => loop {
                match chars.next() {
                    Some((_, '\\')) if c == '"' => {
                        chars.next();
                    }
                    Some((_, q)) if q == c => break,
                    Some(_) => {}
                    None => return Err(ParseWarningKind::UnterminatedQuote),
                }
            },
            _ => {}
        }
    }
    Err(ParseWarningKind::UnterminatedBlock)
}

/// Split tokens into commands at separators.
fn split_commands(tokens: Vec<Token>) -> Result<Vec<CommandLine>, ParseWarningKind> {
    let mut commands = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if token == Token::Separator {
            if current.is_empty() {
                return Err(ParseWarningKind::EmptyCommand);
            }
            commands.push(CommandLine(std::mem::take(&mut current)));
        } else {
            current.push(token.into_text());
        }
    }
    if current.is_empty() {
        return Err(if commands.is_empty() {
            ParseWarningKind::MissingCommand
        } else {
            ParseWarningKind::EmptyCommand
        });
    }
    commands.push(CommandLine(current));
    Ok(commands)
}

/// Read a `{ … }` block argument (as kept in a [`CommandLine`]) as a command
/// list. `None` if `token` is not a well-formed, non-empty block.
pub fn parse_block(token: &str) -> Option<Vec<CommandLine>> {
    let inner = token.strip_prefix('{')?.strip_suffix('}')?;
    if !inner.starts_with(char::is_whitespace) {
        return None;
    }
    split_commands(tokenize(inner).ok()?).ok()
}

/// Mouse keys (`MouseDown1Pane`, `M-WheelUpStatus`, …): the simulator is
/// keyboard-only, so these are skipped without a warning.
fn is_mouse_key(name: &str) -> bool {
    let mut rest = name;
    while let Some(tail) = ["C-", "M-", "S-"].iter().find_map(|p| rest.strip_prefix(p)) {
        rest = tail;
    }
    [
        "Mouse",
        "Wheel",
        "DoubleClick",
        "TripleClick",
        "SecondClick",
    ]
    .iter()
    .any(|p| rest.starts_with(p))
}

enum Line {
    Binding(Binding),
    Mouse,
}

fn parse_line(line: &str) -> Result<Line, ParseWarningKind> {
    let mut tokens = tokenize(line)?.into_iter();
    match tokens.next() {
        Some(Token::Word(w)) if w == "bind-key" || w == "bind" => {}
        _ => return Err(ParseWarningKind::NotBindKey),
    }
    let mut table = "prefix".to_owned();
    let mut repeat = false;
    let key_name = loop {
        let Some(token) = tokens.next() else {
            return Err(ParseWarningKind::MissingKey);
        };
        let Token::Word(w) = &token else {
            break token.into_text();
        };
        match w.as_str() {
            "-r" => repeat = true,
            "-n" => "root".clone_into(&mut table),
            "-T" => match tokens.next() {
                Some(Token::Word(t)) if !t.is_empty() => table = t,
                _ => return Err(ParseWarningKind::MissingTable),
            },
            "-N" => {
                if tokens.next().is_none() {
                    return Err(ParseWarningKind::MissingKey);
                }
            }
            _ => break token.into_text(),
        }
    };
    if is_mouse_key(&key_name) {
        return Ok(Line::Mouse);
    }
    let key: Key = key_name.parse().map_err(ParseWarningKind::BadKey)?;
    let commands = split_commands(tokens.collect())?;
    Ok(Line::Binding(Binding {
        table,
        key,
        repeat,
        commands,
    }))
}

/// Parse `tmux list-keys` output. Blank lines and `#` comments are ignored;
/// mouse bindings are skipped and counted; any other line that cannot be read
/// becomes a warning. A later binding of the same key replaces an earlier one.
pub fn parse_list_keys(text: &str) -> (KeyTables, Vec<ParseWarning>) {
    let mut tables = KeyTables::default();
    let mut warnings = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match parse_line(trimmed) {
            Ok(Line::Binding(b)) => {
                tables
                    .tables
                    .entry(b.table.clone())
                    .or_default()
                    .insert(b.key, b);
            }
            Ok(Line::Mouse) => tables.skipped_mouse += 1,
            Err(kind) => warnings.push(ParseWarning { line: i + 1, kind }),
        }
    }
    (tables, warnings)
}

impl KeyTables {
    /// The binding of `key` in `table`, if any.
    pub fn lookup(&self, table: &str, key: &Key) -> Option<&Binding> {
        self.tables.get(table)?.get(key)
    }

    /// Keys in `table` whose first command starts with the words `command`
    /// (e.g. `["split-window", "-h"]`), in key order.
    pub fn keys_for(&self, table: &str, command: &[&str]) -> Vec<Key> {
        let Some(bindings) = self.tables.get(table) else {
            return Vec::new();
        };
        bindings
            .values()
            .filter(|b| {
                b.commands.first().is_some_and(|c| {
                    c.0.len() >= command.len() && c.0.iter().zip(command).all(|(a, b)| a == b)
                })
            })
            .map(|b| b.key)
            .collect()
    }

    /// Every binding in `table`, in key order (empty for an unknown table).
    pub fn bindings(&self, table: &str) -> impl Iterator<Item = &Binding> {
        self.tables.get(table).into_iter().flat_map(|t| t.values())
    }

    /// Table names, sorted.
    pub fn tables(&self) -> impl Iterator<Item = &str> {
        self.tables.keys().map(String::as_str)
    }

    /// How many mouse bindings were skipped while parsing.
    pub fn skipped_mouse(&self) -> usize {
        self.skipped_mouse
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        s.parse().unwrap()
    }

    #[test]
    fn parses_simple_and_quoted() {
        let (t, w) = parse_list_keys("bind-key -T prefix % split-window -h\nbind-key -T prefix '\"' split-window\nbind-key -r -T prefix Up select-pane -U\n");
        assert!(w.is_empty());
        assert_eq!(
            t.lookup("prefix", &"%".parse().unwrap()).unwrap().commands[0].0,
            vec!["split-window", "-h"]
        );
        assert_eq!(
            t.lookup("prefix", &"\"".parse().unwrap()).unwrap().commands[0].0,
            vec!["split-window"]
        );
        assert!(t.lookup("prefix", &"Up".parse().unwrap()).unwrap().repeat);
    }

    #[test]
    fn splits_command_sequences() {
        let (t, _) = parse_list_keys(
            r#"bind-key -T prefix x confirm-before -p "kill-pane #P? (y/n)" kill-pane"#,
        );
        assert_eq!(
            t.lookup("prefix", &"x".parse().unwrap()).unwrap().commands[0].0[0],
            "confirm-before"
        );
    }

    #[test]
    fn bad_lines_are_warnings() {
        let (_, w) = parse_list_keys("bind-key -T prefix\nnonsense\n");
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn keys_for_finds_split() {
        let (t, _) = parse_list_keys("bind-key -T prefix % split-window -h\n");
        assert_eq!(
            t.keys_for("prefix", &["split-window", "-h"]),
            vec!["%".parse::<Key>().unwrap()]
        );
    }

    #[test]
    fn escaped_separator_splits_commands() {
        let (t, w) =
            parse_list_keys(r#"bind-key -T root M-a select-pane -t = \; send-keys -M "a;b" ';'"#);
        assert!(w.is_empty(), "{w:?}");
        let b = t.lookup("root", &key("M-a")).unwrap();
        assert_eq!(b.commands.len(), 2);
        assert_eq!(b.commands[0].0, vec!["select-pane", "-t", "="]);
        assert_eq!(b.commands[1].0, vec!["send-keys", "-M", "a;b", ";"]);
    }

    #[test]
    fn quoting_rules() {
        let (t, w) = parse_list_keys(
            r#"bind-key -T prefix a cmd 'it\s "raw"' "say \"hi\" \\ \$x" un\quoted ab"c d"e"#,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            t.lookup("prefix", &key("a")).unwrap().commands[0].0,
            vec![
                "cmd",
                r#"it\s "raw""#,
                r#"say "hi" \ $x"#,
                "unquoted",
                "abc de"
            ]
        );
    }

    #[test]
    fn blocks_are_one_token() {
        let (t, w) = parse_list_keys(
            r##"bind-key -T prefix , command-prompt -I "#W" { rename-window "%%" }
bind-key -T root M-b if-shell -F "#{x}" { send-keys -M } { copy-mode -H ; send-keys -X "}" ; run-shell { a { b } } }"##,
        );
        assert!(w.is_empty(), "{w:?}");
        let b = t.lookup("prefix", &key(",")).unwrap();
        assert_eq!(b.commands.len(), 1);
        assert_eq!(
            b.commands[0].0,
            vec!["command-prompt", "-I", "#W", r#"{ rename-window "%%" }"#]
        );
        assert_eq!(
            parse_block(&b.commands[0].0[3]).unwrap(),
            vec![CommandLine(vec!["rename-window".into(), "%%".into()])]
        );
        let b = t.lookup("root", &key("M-b")).unwrap();
        assert_eq!(b.commands.len(), 1);
        let args = &b.commands[0].0;
        assert_eq!(args.len(), 5);
        assert_eq!(args[3], "{ send-keys -M }");
        let inner = parse_block(&args[4]).unwrap();
        assert_eq!(inner.len(), 3);
        assert_eq!(inner[1].0, vec!["send-keys", "-X", "}"]);
        assert_eq!(inner[2].0, vec!["run-shell", "{ a { b } }"]);
        assert_eq!(parse_block("not-a-block"), None);
        assert_eq!(parse_block("{ \"open }"), None);
    }

    #[test]
    fn escaped_and_quoted_keys() {
        let (t, w) = parse_list_keys(
            "bind-key -T prefix \\; last-pane\nbind-key -T prefix \\\" split-window\nbind-key -T prefix \\% split-window -h\nbind-key -T copy-mode \"M-{\" send-keys -X previous-paragraph\nbind-key -T prefix \\~ show-messages\n",
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            t.lookup("prefix", &key(";")).unwrap().commands[0].0,
            vec!["last-pane"]
        );
        assert_eq!(
            t.lookup("prefix", &key("\"")).unwrap().commands[0].0,
            vec!["split-window"]
        );
        assert_eq!(
            t.lookup("prefix", &key("%")).unwrap().commands[0].0,
            vec!["split-window", "-h"]
        );
        assert!(t.lookup("copy-mode", &key("M-{")).is_some());
        assert!(t.lookup("prefix", &key("~")).is_some());
    }

    #[test]
    fn mouse_bindings_are_skipped_silently() {
        let (t, w) = parse_list_keys(
            "bind-key -T root MouseDown1Pane select-pane\nbind-key -T root M-MouseDown3Pane display-menu\nbind-key -T root WheelUpStatus previous-window\nbind-key -T root DoubleClick1Pane x\nbind-key -T root TripleClick1Pane x\nbind-key -T root SecondClick1Pane x\nbind-key -T root C-a send-prefix\n",
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(t.skipped_mouse(), 6);
        assert_eq!(t.tables().collect::<Vec<_>>(), vec!["root"]);
        assert!(t.lookup("root", &key("C-a")).is_some());
    }

    #[test]
    fn flags_comments_and_defaults() {
        let (t, w) = parse_list_keys(
            "# comment\n\n   \nbind-key -n C-a send-prefix\nbind-key -N \"a note\" b list-keys\nbind -T copy-mode - send-keys -X x\n",
        );
        assert!(w.is_empty(), "{w:?}");
        assert!(t.lookup("root", &key("C-a")).is_some());
        assert!(!t.lookup("root", &key("C-a")).unwrap().repeat);
        assert_eq!(t.lookup("prefix", &key("b")).unwrap().table, "prefix");
        assert!(t.lookup("copy-mode", &key("-")).is_some());
        assert_eq!(
            t.tables().collect::<Vec<_>>(),
            vec!["copy-mode", "prefix", "root"]
        );
    }

    #[test]
    fn later_binding_wins() {
        let (t, _) = parse_list_keys("bind-key -T prefix a one\nbind-key -T prefix a two\n");
        assert_eq!(
            t.lookup("prefix", &key("a")).unwrap().commands[0].0,
            vec!["two"]
        );
    }

    #[test]
    fn warning_kinds() {
        let (_, w) = parse_list_keys(
            "bind-key -T prefix Nope cmd\nbind-key -T prefix a\nbind-key -T prefix a cmd \"open\nbind-key -T prefix a cmd { x\nbind-key -T prefix a \\; cmd\nbind-key -T\n",
        );
        let kinds: Vec<_> = w.iter().map(|w| (w.line, w.kind.clone())).collect();
        assert_eq!(kinds.len(), 6, "{kinds:?}");
        assert!(matches!(kinds[0], (1, ParseWarningKind::BadKey(_))));
        assert_eq!(kinds[1], (2, ParseWarningKind::MissingCommand));
        assert_eq!(kinds[2], (3, ParseWarningKind::UnterminatedQuote));
        assert_eq!(kinds[3], (4, ParseWarningKind::UnterminatedBlock));
        assert_eq!(kinds[4], (5, ParseWarningKind::EmptyCommand));
        assert_eq!(kinds[5], (6, ParseWarningKind::MissingTable));
        assert!(w[0].to_string().starts_with("line 1: "));
    }

    #[test]
    fn lookup_and_keys_for_misses() {
        let (t, _) = parse_list_keys(
            "bind-key -T prefix % split-window -h\nbind-key -T prefix \\\" split-window\n",
        );
        assert!(t.lookup("nope", &key("%")).is_none());
        assert!(t.lookup("prefix", &key("a")).is_none());
        assert!(t.keys_for("nope", &["split-window"]).is_empty());
        assert_eq!(t.keys_for("prefix", &["split-window"]).len(), 2);
        assert!(t.keys_for("prefix", &["split-window", "-v"]).is_empty());
        let keys: Vec<Key> = t.bindings("prefix").map(|b| b.key).collect();
        assert_eq!(keys, vec![key("\""), key("%")]);
        assert_eq!(t.bindings("nope").count(), 0);
    }

    #[test]
    fn garbage_does_not_panic() {
        for s in [
            "\\",
            "'",
            "\"",
            "{",
            "}",
            "bind-key",
            "bind-key -r",
            "bind-key -T x \\",
            "bind-key -T x a {",
            "bind-key -T x a }",
            "bind-key -T x a \"\\",
            "bind-key -T x 💥 a",
            "bind-key -T x a 💥\\",
            "\u{0} \u{7f}",
            "bind-key -N",
            "bind-key -T x a \\;",
            "bind-key -T x a ;",
        ] {
            let _ = parse_list_keys(s);
            let _ = parse_block(s);
        }
    }
}
