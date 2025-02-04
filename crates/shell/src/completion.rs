use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow::{self, Owned};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ShellCompleter {
    builtins: HashSet<String>,
}

impl ShellCompleter {
    pub fn new(builtins: HashSet<String>) -> Self {
        Self { builtins }
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
        complete_shell_commands(is_start, &self.builtins, word, &mut matches);

        // Complete executables in PATH
        complete_executables_in_path(is_start, word, &mut matches);

        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches.dedup_by(|a, b| a.display == b.display);

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

fn escape_for_shell(s: &str) -> String {
    let special_chars = [
        ' ', '\'', '"', '(', ')', '[', ']', '|', '&', ';', '<', '>', '$', '`', '\\', '\t', '\n',
        '*', '?', '{', '}', '!',
    ];

    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

#[derive(Debug)]
struct FileMatch {
    name: String,
    #[allow(dead_code)]
    path: PathBuf,
    is_dir: bool,
    is_executable: bool,
    is_symlink: bool,
}

impl FileMatch {
    fn from_entry(entry: fs::DirEntry, base_path: &Path) -> Option<Self> {
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => return None,
        };

        let name = entry.file_name().into_string().ok()?;

        // Skip hidden files
        if name.starts_with('.') {
            return None;
        }

        Some(Self {
            name,
            path: base_path.join(entry.file_name()),
            is_dir: metadata.is_dir(),
            is_executable: is_executable(&entry),
            is_symlink: metadata.file_type().is_symlink(),
        })
    }

    fn replacement(&self, base: &str) -> String {
        let escaped = escape_for_shell(&self.name);
        if self.is_dir {
            format!("{}{}/", base, escaped)
        } else {
            format!("{}{}", base, escaped)
        }
    }

    fn display_name(&self) -> String {
        let mut name = self.name.clone();
        if self.is_dir {
            name.push('/');
        } else if self.is_executable {
            name.push('*');
        }
        if self.is_symlink {
            name.push('@');
        }
        name
    }
}

fn is_executable(entry: &fs::DirEntry) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let Ok(metadata) = entry.metadata() else {
            return false;
        };

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext = ext.to_lowercase();
                matches!(ext.as_str(), "exe" | "bat" | "cmd")
            })
            .unwrap_or(false)
    }
}

fn resolve_dir_path(dir_path: &str) -> PathBuf {
    if dir_path.starts_with('/') {
        PathBuf::from(dir_path)
    } else if let Some(stripped) = dir_path.strip_prefix('~') {
        dirs::home_dir()
            .map(|h| h.join(stripped.strip_prefix('/').unwrap_or(stripped)))
            .unwrap_or_else(|| PathBuf::from(dir_path))
    } else {
        PathBuf::from(".").join(dir_path)
    }
}

fn complete_filenames(is_start: bool, word: &str, matches: &mut Vec<Pair>) {
    let (dir_path, partial_name) = match word.rfind('/') {
        Some(last_slash) => (&word[..=last_slash], &word[last_slash + 1..]),
        None => ("", word),
    };

    let search_dir = resolve_dir_path(dir_path);
    let only_executable = (word.starts_with("./") || word.starts_with('/')) && is_start;

    let files: Vec<FileMatch> = fs::read_dir(&search_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| FileMatch::from_entry(entry, &search_dir))
        .filter(|f| f.name.starts_with(partial_name))
        .filter(|f| !only_executable || f.is_executable || f.is_dir)
        .collect();

    matches.extend(files.into_iter().map(|f| Pair {
        display: f.display_name(),
        replacement: f.replacement(dir_path),
    }));
}

fn complete_shell_commands(
    is_start: bool,
    builtin_commands: &HashSet<String>,
    word: &str,
    matches: &mut Vec<Pair>,
) {
    if !is_start {
        return;
    }

    for cmd in builtin_commands {
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
    let mut found = HashSet::new();
    if let Ok(paths) = env::var("PATH") {
        for path in env::split_paths(&paths) {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(word)
                            && entry.path().is_file()
                            && found.insert(name.clone())
                        {
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
