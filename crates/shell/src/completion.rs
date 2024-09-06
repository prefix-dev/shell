use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow::{self, Owned};
use std::env;
use std::fs;

pub struct ShellCompleter;

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut matches = Vec::new();
        let (start, word) = extract_word(line, pos);

        // Complete filenames
        complete_filenames(word, &mut matches);

        // Complete shell commands
        complete_shell_commands(word, &mut matches);

        // Complete executables in PATH
        complete_executables_in_path(word, &mut matches);

        Ok((start, matches))
    }
}

fn extract_word(line: &str, pos: usize) -> (usize, &str) {
    let words: Vec<_> = line[..pos].split_whitespace().collect();
    let word_start = words.last().map_or(0, |w| line.rfind(w).unwrap());
    (word_start, &line[word_start..pos])
}

fn complete_filenames(word: &str, matches: &mut Vec<Pair>) {
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with(word) {
                    matches.push(Pair {
                        display: name.clone(),
                        replacement: name,
                    });
                }
            }
        }
    }
}

fn complete_shell_commands(word: &str, matches: &mut Vec<Pair>) {
    let shell_commands = ["ls", "cat", "cd", "pwd", "echo", "grep"];
    for &cmd in &shell_commands {
        if cmd.starts_with(word) {
            matches.push(Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
            });
        }
    }
}

fn complete_executables_in_path(word: &str, matches: &mut Vec<Pair>) {
    if let Ok(paths) = env::var("PATH") {
        for path in env::split_paths(&paths) {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(word) && entry.path().is_file() {
                            matches.push(Pair {
                                display: name.clone(),
                                replacement: name,
                            });
                        }
                    }
                }
            }
        }
    }
}

impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }
}

impl Validator for ShellCompleter {}

impl Helper for ShellCompleter {}
