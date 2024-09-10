// Copyright 2018-2024 the Deno authors. MIT license.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use futures::future::LocalBoxFuture;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::shell::fs_util;

use super::commands::builtin_commands;
use super::commands::ShellCommand;

#[derive(Clone)]
pub struct ShellState {
  /// Environment variables that should be passed down to sub commands
  /// and used when evaluating environment variables.
  env_vars: HashMap<String, String>,
  /// Variables that should be evaluated within the shell and
  /// not passed down to any sub commands.
  shell_vars: HashMap<String, String>,
  /// The current working directory of the shell
  cwd: PathBuf,
  /// The commands that are available in the shell
  commands: Rc<HashMap<String, Rc<dyn ShellCommand>>>,
  /// A map of aliases for commands (e.g. `ll=ls -al`)
  alias: HashMap<String, Vec<String>>,
  /// Token to cancel execution.
  token: CancellationToken,
  /// Git repository handling.
  git_repository: bool, // Is `cwd` inside a git repository?
  git_root: PathBuf, // Path to the root (`$git_root/.git/HEAD` exists)
  git_branch: String, // Contents of `$git_root/.git/HEAD`
  last_command_cd: bool, // Was last command a `cd` (thus git_branch is current)?
}

impl ShellState {
  pub fn new(
    env_vars: HashMap<String, String>,
    cwd: &Path,
    custom_commands: HashMap<String, Rc<dyn ShellCommand>>,
  ) -> Self {
    assert!(cwd.is_absolute());
    let mut commands = builtin_commands();
    commands.extend(custom_commands);
    let mut result = Self {
      env_vars: Default::default(),
      shell_vars: Default::default(),
      alias: Default::default(),
      cwd: PathBuf::new(),
      commands: Rc::new(commands),
      token: CancellationToken::default(),
      git_repository: false,
      git_root: PathBuf::new(),
      git_branch: String::new(),
      last_command_cd: false,
    };
    // ensure the data is normalized
    for (name, value) in env_vars {
      result.apply_env_var(&name, &value);
    }
    result.set_cwd(cwd);
    result
  }

  pub fn cwd(&self) -> &PathBuf {
    &self.cwd
  }

  pub fn alias_map(&self) -> &HashMap<String, Vec<String>> {
    &self.alias
  }

  pub fn git_repository(&self) -> bool {
    self.git_repository
  }

  pub fn git_root(&self) -> &PathBuf {
    &self.git_root
  }

  pub fn git_branch(&self) -> &String {
    &self.git_branch
  }

  pub fn last_command_cd(&self) -> bool {
    self.last_command_cd
  }

  pub fn env_vars(&self) -> &HashMap<String, String> {
    &self.env_vars
  }

  pub fn get_var(&self, name: &str) -> Option<&String> {
    let name = if cfg!(windows) {
      Cow::Owned(name.to_uppercase())
    } else {
      Cow::Borrowed(name)
    };
    self
      .env_vars
      .get(name.as_ref())
      .or_else(|| self.shell_vars.get(name.as_ref()))
  }

  // Update self.git_branch using self.git_root
  pub fn update_git_branch(&mut self) {
    if self.git_repository {
      match fs::read_to_string(self.git_root().join(".git/HEAD")) {
        Ok(contents) => {
          // The git root can still be read, update the git branch
          self.git_branch = contents.trim().to_string();
        }
        Err(_) => {
          // The git root can no longer be read
          // (the `.git/HEAD` was removed in the meantime)
          self.git_repository = false;
          self.git_branch = "".to_string();
          self.git_root = "".to_string().into();
        }
      };
    }
  }

  /// Set the current working directory of this shell
  pub fn set_cwd(&mut self, cwd: &Path) {
    self.cwd = cwd.to_path_buf();
    // $PWD holds the current working directory, so we keep cwd and $PWD in sync
    self
      .env_vars
      .insert("PWD".to_string(), self.cwd.display().to_string());
    // Handle a git repository
    // First read the current directory's `.git/HEAD`
    match fs::read_to_string(cwd.join(".git/HEAD")) {
      Ok(contents) => {
        // We are in a git repository in the git root dir
        self.git_repository = true;
        self.git_branch = contents.trim().to_string();
        self.git_root = cwd.to_path_buf();
      }
      Err(_) => {
        if self.git_repository
          && cwd
            .display()
            .to_string()
            .starts_with(&self.git_root.display().to_string())
        {
          // We moved inside the same git repository, but we are not
          // in the git root dir
          self.update_git_branch();
        } else {
          // We didn't move within the same git repository,
          // and there is no `.git` present.
          // Consequently, we:
          // * Either moved into a subdirectory of a git repository from
          // outside
          // * Or moved into a directory that is not inside git repository
          // In the first case we need to recursively search to find the
          // root. This might be slow, so we want to be smart and use the
          // old directory to eliminate search in case we are moving up or
          // down from the same root. For now we will set no git
          // repository, which is incorrect for the first case, but will
          // be fast for the most common use of not being inside a git
          // repository.
          self.git_repository = false;
          self.git_branch = "".to_string();
          self.git_root = "".to_string().into();
        }
      }
    };
  }

  pub fn apply_changes(&mut self, changes: &[EnvChange]) {
    self.last_command_cd = false;
    for change in changes {
      self.apply_change(change);
    }
  }

  pub fn apply_change(&mut self, change: &EnvChange) {
    match change {
      EnvChange::SetEnvVar(name, value) => self.apply_env_var(name, value),
      EnvChange::SetShellVar(name, value) => {
        if self.env_vars.contains_key(name) {
          self.apply_env_var(name, value);
        } else {
          self.shell_vars.insert(name.to_string(), value.to_string());
        }
      }
      EnvChange::UnsetVar(name) => {
        self.shell_vars.remove(name);
        self.env_vars.remove(name);
      }
      EnvChange::Cd(new_dir) => {
        self.set_cwd(new_dir);
        self.last_command_cd = true;
      }
      EnvChange::AliasCommand(alias, cmd) => {
        self.alias.insert(
          alias.clone(),
          cmd.split_whitespace().map(ToString::to_string).collect(),
        );
      },
      EnvChange::UnAliasCommand(alias) => {
        println!("unalias {}", alias);
        self.alias.remove(alias);
      }
    }
  }

  pub fn apply_env_var(&mut self, name: &str, value: &str) {
    let name = if cfg!(windows) {
      // environment variables are case insensitive on windows
      name.to_uppercase()
    } else {
      name.to_string()
    };
    if name == "PWD" {
      let cwd = PathBuf::from(value);
      if cwd.is_absolute() {
        if let Ok(cwd) = fs_util::canonicalize_path(&cwd) {
          // this will update the environment variable too
          self.set_cwd(&cwd);
        }
      }
    } else {
      self.shell_vars.remove(&name);
      self.env_vars.insert(name, value.to_string());
    }
  }

  pub fn token(&self) -> &CancellationToken {
    &self.token
  }

  /// Resolves a custom command that was injected.
  pub fn resolve_custom_command(
    &self,
    name: &str,
  ) -> Option<Rc<dyn ShellCommand>> {
    // uses an Rc to allow resolving a command without borrowing from self
    self.commands.get(name).cloned()
  }

  /// Resolves the path to a command from the current working directory.
  ///
  /// Does not take injected custom commands into account.
  pub fn resolve_command_path(
    &self,
    command_name: &str,
  ) -> Result<PathBuf, crate::ResolveCommandPathError> {
    super::command::resolve_command_path(command_name, self.cwd(), self)
  }

  pub fn with_child_token(&self) -> ShellState {
    let mut state = self.clone();
    state.token = self.token.child_token();
    state
  }

  pub fn reset_cancellation_token(&mut self) {
    self.token = CancellationToken::default();
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnvChange {
  /// `export ENV_VAR=VALUE`
  SetEnvVar(String, String),
  /// `ENV_VAR=VALUE`
  SetShellVar(String, String),
  /// Create an alias for a command (e.g. ll=ls -al)
  AliasCommand(String, String),
  /// Remove an alias
  UnAliasCommand(String),
  /// `unset ENV_VAR`
  UnsetVar(String),
  /// Set the current working directory to the new Path
  Cd(PathBuf),
}

pub type FutureExecuteResult = LocalBoxFuture<'static, ExecuteResult>;

// https://unix.stackexchange.com/a/99117
// SIGINT (2) + 128
pub const CANCELLATION_EXIT_CODE: i32 = 130;

#[derive(Debug)]
pub enum ExecuteResult {
  Exit(i32, Vec<JoinHandle<i32>>),
  Continue(i32, Vec<EnvChange>, Vec<JoinHandle<i32>>),
}

impl ExecuteResult {
  pub fn for_cancellation() -> ExecuteResult {
    ExecuteResult::Exit(CANCELLATION_EXIT_CODE, Vec::new())
  }

  pub fn from_exit_code(exit_code: i32) -> ExecuteResult {
    ExecuteResult::Continue(exit_code, Vec::new(), Vec::new())
  }

  pub fn into_exit_code_and_handles(self) -> (i32, Vec<JoinHandle<i32>>) {
    match self {
      ExecuteResult::Exit(code, handles) => (code, handles),
      ExecuteResult::Continue(code, _, handles) => (code, handles),
    }
  }

  pub fn into_handles(self) -> Vec<JoinHandle<i32>> {
    self.into_exit_code_and_handles().1
  }
}

/// Reader side of a pipe.
#[derive(Debug)]
pub enum ShellPipeReader {
  OsPipe(os_pipe::PipeReader),
  StdFile(std::fs::File),
}

impl Clone for ShellPipeReader {
  fn clone(&self) -> Self {
    match self {
      Self::OsPipe(pipe) => Self::OsPipe(pipe.try_clone().unwrap()),
      Self::StdFile(file) => Self::StdFile(file.try_clone().unwrap()),
    }
  }
}

impl ShellPipeReader {
  pub fn stdin() -> ShellPipeReader {
    ShellPipeReader::from_raw(os_pipe::dup_stdin().unwrap())
  }

  pub fn from_raw(reader: os_pipe::PipeReader) -> Self {
    Self::OsPipe(reader)
  }

  pub fn from_std(std_file: std::fs::File) -> Self {
    Self::StdFile(std_file)
  }

  pub fn into_stdio(self) -> std::process::Stdio {
    match self {
      Self::OsPipe(pipe) => pipe.into(),
      Self::StdFile(file) => file.into(),
    }
  }

  /// Pipe everything to the specified writer
  pub fn pipe_to(self, writer: &mut dyn Write) -> Result<()> {
    // don't bother flushing here because this won't ever be called
    // with a Rust wrapped stdout/stderr
    self.pipe_to_inner(writer, false)
  }

  fn pipe_to_with_flushing(self, writer: &mut dyn Write) -> Result<()> {
    self.pipe_to_inner(writer, true)
  }

  fn pipe_to_inner(
    mut self,
    writer: &mut dyn Write,
    flush: bool,
  ) -> Result<()> {
    loop {
      let mut buffer = [0; 512]; // todo: what is an appropriate buffer size?
      let size = match &mut self {
        ShellPipeReader::OsPipe(pipe) => pipe.read(&mut buffer)?,
        ShellPipeReader::StdFile(file) => file.read(&mut buffer)?,
      };
      if size == 0 {
        break;
      }
      writer.write_all(&buffer[0..size])?;
      if flush {
        writer.flush()?;
      }
    }
    Ok(())
  }

  /// Pipes this pipe to the specified sender.
  pub fn pipe_to_sender(self, mut sender: ShellPipeWriter) -> Result<()> {
    match &mut sender {
      ShellPipeWriter::OsPipe(pipe) => self.pipe_to(pipe),
      ShellPipeWriter::StdFile(file) => self.pipe_to(file),
      // Don't lock stdout/stderr here because we want to release the lock
      // when reading from the sending pipe. Additionally, we want
      // to flush after every write because Rust's wrapper has an
      // internal buffer and Deno doesn't buffer stdout/stderr.
      ShellPipeWriter::Stdout => {
        self.pipe_to_with_flushing(&mut std::io::stdout())
      }
      ShellPipeWriter::Stderr => {
        self.pipe_to_with_flushing(&mut std::io::stderr())
      }
      ShellPipeWriter::Null => Ok(()),
    }
  }

  /// Pipes the reader to a string handle that is resolved when the pipe's
  /// writer is closed.
  pub fn pipe_to_string_handle(self) -> JoinHandle<String> {
    tokio::task::spawn_blocking(|| {
      let mut buf = Vec::new();
      self.pipe_to(&mut buf).unwrap();
      String::from_utf8_lossy(&buf).to_string()
    })
  }

  pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
    match self {
      ShellPipeReader::OsPipe(pipe) => pipe.read(buf).map_err(|e| e.into()),
      ShellPipeReader::StdFile(file) => file.read(buf).map_err(|e| e.into()),
    }
  }
}

/// Writer side of a pipe.
///
/// Ensure that all of these are dropped when complete in order to
/// prevent deadlocks where the reader hangs waiting for a read.
#[derive(Debug)]
pub enum ShellPipeWriter {
  OsPipe(os_pipe::PipeWriter),
  StdFile(std::fs::File),
  // For stdout and stderr, instead of directly duplicating the raw pipes
  // and putting them in a ShellPipeWriter::OsPipe(...), we use Rust std's
  // stdout() and stderr() wrappers because it contains some code to solve
  // some encoding issues on Windows (ex. emojis). For more details, see
  // library/std/src/sys/windows/stdio.rs in Rust's source code.
  Stdout,
  Stderr,
  Null,
}

impl Clone for ShellPipeWriter {
  fn clone(&self) -> Self {
    match self {
      Self::OsPipe(pipe) => Self::OsPipe(pipe.try_clone().unwrap()),
      Self::StdFile(file) => Self::StdFile(file.try_clone().unwrap()),
      Self::Stdout => Self::Stdout,
      Self::Stderr => Self::Stderr,
      Self::Null => Self::Null,
    }
  }
}

impl ShellPipeWriter {
  pub fn stdout() -> Self {
    Self::Stdout
  }

  pub fn stderr() -> Self {
    Self::Stderr
  }

  pub fn null() -> Self {
    Self::Null
  }

  pub fn from_std(std_file: std::fs::File) -> Self {
    Self::StdFile(std_file)
  }

  pub fn into_stdio(self) -> std::process::Stdio {
    match self {
      Self::OsPipe(pipe) => pipe.into(),
      Self::StdFile(file) => file.into(),
      Self::Stdout => std::process::Stdio::inherit(),
      Self::Stderr => std::process::Stdio::inherit(),
      Self::Null => std::process::Stdio::null(),
    }
  }

  pub fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
    match self {
      Self::OsPipe(pipe) => pipe.write_all(bytes)?,
      Self::StdFile(file) => file.write_all(bytes)?,
      // For both stdout & stderr, we want to flush after each
      // write in order to bypass Rust's internal buffer.
      Self::Stdout => {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(bytes)?;
        stdout.flush()?;
      }
      Self::Stderr => {
        let mut stderr = std::io::stderr().lock();
        stderr.write_all(bytes)?;
        stderr.flush()?;
      }
      Self::Null => {}
    }
    Ok(())
  }

  pub fn write_line(&mut self, line: &str) -> Result<()> {
    let bytes = format!("{line}\n");
    self.write_all(bytes.as_bytes())
  }
}

/// Used to communicate between commands.
pub fn pipe() -> (ShellPipeReader, ShellPipeWriter) {
  let (reader, writer) = os_pipe::pipe().unwrap();
  (
    ShellPipeReader::OsPipe(reader),
    ShellPipeWriter::OsPipe(writer),
  )
}
