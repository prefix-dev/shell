use rustyline::{
    completion::{Completer, FilenameCompleter},
    highlight::{Highlighter, MatchingBracketHighlighter},
    hint::{Hinter, HistoryHinter},
    validate::{MatchingBracketValidator, Validator},
    Completer, Helper, Hinter, Validator,
};
use std::borrow::Cow;
use std::borrow::Cow::{Borrowed, Owned};

#[derive(Helper, Completer, Hinter, Validator)]
pub struct MyHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,

    highlighter: MatchingBracketHighlighter,

    #[rustyline(Validator)]
    validator: MatchingBracketValidator,

    #[rustyline(Hinter)]
    hinter: HistoryHinter,

    colored_prompt: String,
}

impl Default for MyHelper {
    fn default() -> Self {
        Self {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter::new(),
            colored_prompt: "\x1b[1;32m>>>\x1b[m ".to_owned(),
        }
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}
