use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow::{self, Owned};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct ShellCompleter {
    generic_completions: HashMap<String, serde_json::Value>,
}

impl ShellCompleter {
    pub fn new() -> Self {
        let mut generic_completions = HashMap::new();

        let contents = include_str!("../data/completions/git.json");
        if let Ok(json) = serde_json::from_str(&contents) {
            if let serde_json::Value::Object(map) = json {
                for (key, value) in map {
                    generic_completions.insert(key, value);
                }
            }
        }

        ShellCompleter {
            generic_completions,
        }
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

        let parts: Vec<&str> = line[..pos].split_whitespace().collect();
        // Complete generic commands (including git)
        if !parts.is_empty() && self.generic_completions.contains_key(parts[0]) {
            complete_generic_commands(self, line, pos, &mut matches);
            let start = line[..pos].rfind(char::is_whitespace).map_or(0, |i| i + 1);
            return Ok((start, matches));
        }

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
    if line.ends_with(" ") {
        return (pos, "");
    }
    let words: Vec<_> = line[..pos].split_whitespace().collect();
    let word_start = words.last().map_or(0, |w| line.rfind(w).unwrap());
    (word_start, &line[word_start..pos])
}

fn complete_filenames(_is_start: bool, word: &str, matches: &mut Vec<Pair>) {
    // Split the word into directory path and partial filename
    let (dir_path, partial_name) = match word.rfind('/') {
        Some(last_slash) => (&word[..=last_slash], &word[last_slash + 1..]),
        None => ("", word),
    };

    // Determine the full directory path to search
    let search_dir = if dir_path.starts_with('/') {
        dir_path.to_string()
    } else {
        format!("./{}", dir_path)
    };

    if let Ok(entries) = fs::read_dir(Path::new(&search_dir)) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with(partial_name) {
                    let full_path = format!("{}{}", dir_path, name);
                    match entry.file_type() {
                        Ok(file_type) if file_type.is_dir() => {
                            matches.push(Pair {
                                display: full_path.clone() + "/",
                                replacement: full_path + "/",
                            });
                        }
                        Ok(_) => {
                            matches.push(Pair {
                                display: full_path.clone(),
                                replacement: full_path,
                            });
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
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

fn complete_generic_commands(completer: &ShellCompleter, line: &str, pos: usize, matches: &mut Vec<Pair>) {
    let parts: Vec<&str> = line[..pos].split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let command = parts[0];
    if let Some(completions) = completer.generic_completions.get(command) {
        let mut current = completions;
        let mut partial = "";

        for (i, part) in parts.iter().enumerate().skip(1) {
            if i == parts.len() - 1 && !line.ends_with(" ") {
                partial = part;
                break;
            }

            if let Some(next) = current.get(part) {
                current = next;
            } else {
                return;
            }
        }

        if let Some(default) = current.get("$default") {
            current = default;
        }

        match current {
            serde_json::Value::Object(map) => {
                if let Some(exec) = map.get("$exec") {
                    if let Some(cmd) = exec.as_str() {
                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(cmd)
                            .output()
                            .expect("Failed to execute command");
                        let completions = String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .filter(|s| s.starts_with(partial))
                            .map(|s| Pair {
                                display: s.to_string(),
                                replacement: s.to_string(),
                            })
                            .collect::<Vec<_>>();
                        matches.extend(completions);
                    }
                } else if let Some(input) = map.get("$input") {
                    if let Some(prompt) = input.as_str() {
                        println!("{}", prompt);
                    }
                } else {
                    for key in map.keys() {
                        if key.starts_with(partial) && *key != "$exec" && *key != "$input" && *key != "$default" {
                            matches.push(Pair {
                                display: key.clone(),
                                replacement: key.clone(),
                            });
                        }
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if s.starts_with(partial) {
                            matches.push(Pair {
                                display: s.to_string(),
                                replacement: s.to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
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
