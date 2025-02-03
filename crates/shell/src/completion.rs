use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow::{self, Owned};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

pub struct ShellCompleter;

impl Default for ShellCompleter {
    fn default() -> Self {
        ShellCompleter
    }
}

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

        let is_start = start == 0;
        // Complete filenames
        complete_filenames(is_start, word, &mut matches);

        // Complete shell commands
        complete_shell_commands(is_start, word, &mut matches);

        // Complete executables in PATH
        complete_executables_in_path(is_start, word, &mut matches);

        Ok((start, matches))
    }
}

fn extract_word(line: &str, pos: usize) -> (usize, &str) {
    if line.ends_with(' ') {
        return (pos, "");
    }
    let words: Vec<_> = line[..pos].split_whitespace().collect();
    let word_start = words.last().map_or(0, |w| line.rfind(w).unwrap());
    (word_start, &line[word_start..pos])
}

fn complete_filenames(is_start: bool, word: &str, matches: &mut Vec<Pair>) {
    let only_executable = word.starts_with("./") && is_start;

    // Split the word into directory path and partial filename
    let (dir_path, partial_name) = match word.rfind('/') {
        Some(last_slash) => (&word[..=last_slash], &word[last_slash + 1..]),
        None => ("", word),
    };

    // Determine the full directory path to search
    let search_dir = if dir_path.starts_with('/') {
        dir_path.to_string()
    } else if let Some(stripped) = dir_path.strip_prefix('~') {
        let home_dir = dirs::home_dir().unwrap();
        format!("{}{}", home_dir.display(), stripped)
    } else {
        format!("./{}", dir_path)
    };

    let mut matching = Vec::new();
    if let Ok(entries) = fs::read_dir(Path::new(&search_dir)) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with(partial_name) {
                    let full_path = format!("{}{}", dir_path, name);
                    match entry.file_type() {
                        Ok(file_type) if file_type.is_dir() => {
                            // only display last path component
                            let display = if let Some(stripped) = full_path.rsplit('/').next() {
                                stripped.to_owned()
                            } else {
                                full_path.clone()
                            };

                            matching.push(Pair {
                                display: display + "/",
                                replacement: full_path + "/",
                            });
                        }
                        Ok(_) => {
                            let is_executable =
                                entry.metadata().unwrap().permissions().mode() & 0o111 != 0;
                            if only_executable && !is_executable {
                                continue;
                            }

                            // Only display last path component
                            let mut display = if let Some(stripped) = full_path.rsplit('/').next() {
                                stripped.to_owned()
                            } else {
                                full_path.clone()
                            };

                            if is_executable {
                                display.push_str("*");
                            }

                            if entry.metadata().unwrap().permissions().mode() & 0o111 != 0 {
                                matching.push(Pair {
                                    display,
                                    replacement: full_path,
                                });
                            } else {
                                matching.push(Pair {
                                    display,
                                    replacement: full_path,
                                });
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
    // sort matches
    matching.sort_by(|a, b| a.display.cmp(&b.display));
    matches.extend(matching);
}

fn complete_shell_commands(is_start: bool, word: &str, matches: &mut Vec<Pair>) {
    if !is_start {
        return;
    }
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

fn complete_executables_in_path(is_start: bool, word: &str, matches: &mut Vec<Pair>) {
    if !is_start {
        return;
    }
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
