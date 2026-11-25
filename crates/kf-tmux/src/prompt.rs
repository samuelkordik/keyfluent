//! The client's status-line prompts: `command-prompt` and `confirm-before`.

use kf_core::bindings::{parse_command_line, CommandLine};
use kf_core::key::{Key, KeyBase, NamedKey};

use crate::commands::PromptRequest;

/// An open prompt on the status line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prompt {
    /// `command-prompt`: an editable input line.
    Input {
        label: String,
        input: String,
        /// Run with `%%` replaced by the input; empty = run the input itself
        /// as a command line (the `:` prompt).
        template: Vec<CommandLine>,
    },
    /// `confirm-before`: `y` runs `then`, any other key cancels.
    Confirm {
        prompt: String,
        then: Vec<CommandLine>,
    },
}

/// What a key press did to a prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptStep {
    /// The prompt stays open.
    Open,
    /// The prompt closed without running anything.
    Closed,
    /// The prompt closed; run these commands.
    Run(Vec<CommandLine>),
    /// The prompt closed; the typed command line could not be read.
    Failed(String),
}

impl From<PromptRequest> for Prompt {
    fn from(r: PromptRequest) -> Prompt {
        Prompt::Input {
            label: r.prompt,
            input: r.initial,
            template: r.template,
        }
    }
}

impl Prompt {
    /// (label, input) as the status line shows them.
    pub fn status(&self) -> (String, String) {
        match self {
            Prompt::Input { label, input, .. } => (label.clone(), input.clone()),
            Prompt::Confirm { prompt, .. } => (prompt.clone(), String::new()),
        }
    }

    /// Handles one key. Input editing: a printable character without `C-`/`M-`
    /// inserts, `Space` inserts a space, `BSpace` deletes the last character,
    /// `C-u` clears, `Enter` submits, `Escape` and `C-c` cancel; other keys are
    /// ignored. Submitting an empty input runs nothing (tmux ignores an
    /// empty rename).
    pub fn key(&mut self, k: &Key) -> PromptStep {
        let plain = !k.ctrl && !k.meta;
        match self {
            Prompt::Confirm { then, .. } => {
                if plain && k.base == KeyBase::Char('y') {
                    PromptStep::Run(std::mem::take(then))
                } else {
                    PromptStep::Closed
                }
            }
            Prompt::Input {
                input, template, ..
            } => match (k.base, k.ctrl, k.meta) {
                (KeyBase::Char(c), false, false) if !c.is_control() => {
                    input.push(c);
                    PromptStep::Open
                }
                (KeyBase::Named(NamedKey::Space), false, false) => {
                    input.push(' ');
                    PromptStep::Open
                }
                (KeyBase::Named(NamedKey::BSpace), false, false) => {
                    input.pop();
                    PromptStep::Open
                }
                (KeyBase::Char('u'), true, false) => {
                    input.clear();
                    PromptStep::Open
                }
                (KeyBase::Named(NamedKey::Escape), _, _) | (KeyBase::Char('c'), true, false) => {
                    PromptStep::Closed
                }
                (KeyBase::Named(NamedKey::Enter), false, false) => submit(input, template),
                _ => PromptStep::Open,
            },
        }
    }
}

fn submit(input: &str, template: &[CommandLine]) -> PromptStep {
    if template.is_empty() {
        if input.trim().is_empty() {
            return PromptStep::Closed;
        }
        return match parse_command_line(input) {
            Ok(commands) => PromptStep::Run(commands),
            Err(e) => PromptStep::Failed(e.to_string()),
        };
    }
    if input.is_empty() {
        return PromptStep::Closed;
    }
    PromptStep::Run(substitute(template, input))
}

/// `template` with every `%%` and `%1` in its arguments replaced by `input`.
pub fn substitute(template: &[CommandLine], input: &str) -> Vec<CommandLine> {
    template
        .iter()
        .map(|c| CommandLine(c.0.iter().map(|w| substitute_word(w, input)).collect()))
        .collect()
}

/// One left-to-right pass, so placeholders typed in `input` stay literal.
fn substitute_word(word: &str, input: &str) -> String {
    let mut out = String::new();
    let mut rest = word;
    while let Some(pos) = rest.find('%') {
        out.push_str(rest.get(..pos).unwrap_or_default());
        let after = rest.get(pos + 1..).unwrap_or_default();
        if let Some(tail) = after.strip_prefix('%').or_else(|| after.strip_prefix('1')) {
            out.push_str(input);
            rest = tail;
        } else {
            out.push('%');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kf_core::key::parse_keys;

    fn cmd(s: &str) -> CommandLine {
        CommandLine(s.split(' ').map(str::to_owned).collect())
    }

    fn type_keys(p: &mut Prompt, keys: &str) -> PromptStep {
        let mut last = PromptStep::Open;
        for k in parse_keys(keys).unwrap() {
            last = p.key(&k);
        }
        last
    }

    fn rename(initial: &str) -> Prompt {
        Prompt::from(PromptRequest {
            prompt: "(rename-window) ".into(),
            initial: initial.into(),
            template: vec![cmd("rename-window %%")],
        })
    }

    #[test]
    fn substitutes_every_placeholder() {
        assert_eq!(
            substitute(&[cmd("a %% b%%c %1"), cmd("x")], "in"),
            vec![cmd("a in binc in"), cmd("x")]
        );
        // Placeholders in the input are not substituted again.
        assert_eq!(
            substitute(&[cmd("rename-window %%")], "a%1"),
            vec![cmd("rename-window a%1")]
        );
        assert_eq!(
            substitute(&[cmd("a%2 %%% 50%")], "x"),
            vec![cmd("a%2 x% 50%")]
        );
    }

    #[test]
    fn editing_keys() {
        let mut p = rename("bash");
        assert_eq!(
            type_keys(&mut p, "BSpace Space x C-x M-y F1 Up"),
            PromptStep::Open
        );
        assert_eq!(p.status(), ("(rename-window) ".into(), "bas x".into()));
        assert_eq!(type_keys(&mut p, "C-u"), PromptStep::Open);
        assert_eq!(p.status().1, "");
        assert_eq!(type_keys(&mut p, "BSpace"), PromptStep::Open);
        assert_eq!(
            type_keys(&mut p, "l o g s Enter"),
            PromptStep::Run(vec![cmd("rename-window logs")])
        );
        assert_eq!(type_keys(&mut rename("x"), "Escape"), PromptStep::Closed);
        assert_eq!(type_keys(&mut rename("x"), "C-c"), PromptStep::Closed);
        assert_eq!(type_keys(&mut rename("x"), "C-u Enter"), PromptStep::Closed);
    }

    #[test]
    fn command_line_prompt() {
        let colon = || {
            Prompt::from(PromptRequest {
                prompt: ":".into(),
                initial: String::new(),
                template: vec![],
            })
        };
        assert_eq!(
            type_keys(&mut colon(), "s p l i t w Space - h Enter"),
            PromptStep::Run(vec![cmd("splitw -h")])
        );
        assert_eq!(type_keys(&mut colon(), "Space Enter"), PromptStep::Closed);
        assert_eq!(
            type_keys(&mut colon(), "x Space \" Enter"),
            PromptStep::Failed("unterminated quote".into())
        );
    }

    #[test]
    fn confirm() {
        let confirm = || Prompt::Confirm {
            prompt: "kill? ".into(),
            then: vec![cmd("kill-pane")],
        };
        assert_eq!(confirm().status(), ("kill? ".into(), String::new()));
        assert_eq!(
            type_keys(&mut confirm(), "y"),
            PromptStep::Run(vec![cmd("kill-pane")])
        );
        for k in ["n", "Y", "C-y", "Enter", "Escape"] {
            assert_eq!(type_keys(&mut confirm(), k), PromptStep::Closed, "{k}");
        }
    }
}
