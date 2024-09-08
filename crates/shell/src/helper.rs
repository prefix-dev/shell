use rustyline::{
    highlight::Highlighter, validate::MatchingBracketValidator, Completer, Helper, Hinter,
    Validator,
};

use crate::completion;

use std::borrow::Cow::Borrowed;

#[derive(Helper, Completer, Hinter, Validator)]
pub(crate) struct ShellPromptHelper {
    #[rustyline(Completer)]
    completer: completion::ShellCompleter,

    #[rustyline(Validator)]
    validator: MatchingBracketValidator,

    pub colored_prompt: String,
}

impl Default for ShellPromptHelper {
    fn default() -> Self {
        Self {
            completer: completion::ShellCompleter,
            validator: MatchingBracketValidator::new(),
            colored_prompt: String::new(),
        }
    }
}

impl Highlighter for ShellPromptHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }
}
