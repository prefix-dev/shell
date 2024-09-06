// Copyright 2018-2024 the Deno authors. MIT license.

use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

// Shell grammar rules this is loosely based on:
// https://pubs.opengroup.org/onlinepubs/009604499/utilities/xcu_chap02.html#tag_02_10_02

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequentialList {
  pub items: Vec<SequentialListItem>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequentialListItem {
  pub is_async: bool,
  pub sequence: Sequence,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sequence {
  /// `MY_VAR=5`
  ShellVar(EnvVar),
  /// `cmd_name <args...>`, `cmd1 | cmd2`
  Pipeline(Pipeline),
  /// `cmd1 && cmd2 || cmd3`
  BooleanList(Box<BooleanList>),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
  /// `! pipeline`
  pub negated: bool,
  pub inner: PipelineInner,
}

impl From<Pipeline> for Sequence {
  fn from(p: Pipeline) -> Self {
    Sequence::Pipeline(p)
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineInner {
  /// Ex. `cmd_name <args...>`
  Command(Command),
  /// `cmd1 | cmd2`
  PipeSequence(Box<PipeSequence>),
}

impl From<PipeSequence> for PipelineInner {
  fn from(p: PipeSequence) -> Self {
    PipelineInner::PipeSequence(Box::new(p))
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BooleanListOperator {
  // &&
  And,
  // ||
  Or,
}

impl BooleanListOperator {
  pub fn as_str(&self) -> &'static str {
    match self {
      BooleanListOperator::And => "&&",
      BooleanListOperator::Or => "||",
    }
  }

  pub fn moves_next_for_exit_code(&self, exit_code: i32) -> bool {
    *self == BooleanListOperator::Or && exit_code != 0
      || *self == BooleanListOperator::And && exit_code == 0
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanList {
  pub current: Sequence,
  pub op: BooleanListOperator,
  pub next: Sequence,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PipeSequenceOperator {
  // |
  Stdout,
  // |&
  StdoutStderr,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipeSequence {
  pub current: Command,
  pub op: PipeSequenceOperator,
  pub next: PipelineInner,
}

impl From<PipeSequence> for Sequence {
  fn from(p: PipeSequence) -> Self {
    Sequence::Pipeline(Pipeline {
      negated: false,
      inner: p.into(),
    })
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
  pub inner: CommandInner,
  pub redirect: Option<Redirect>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandInner {
  /// `cmd_name <args...>`
  Simple(SimpleCommand),
  /// `(list)`
  Subshell(Box<SequentialList>),
}

impl From<Command> for Sequence {
  fn from(c: Command) -> Self {
    Pipeline {
      negated: false,
      inner: c.into(),
    }
    .into()
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
  pub env_vars: Vec<EnvVar>,
  pub args: Vec<Word>,
}

impl From<SimpleCommand> for Command {
  fn from(c: SimpleCommand) -> Self {
    Command {
      redirect: None,
      inner: CommandInner::Simple(c),
    }
  }
}

impl From<SimpleCommand> for PipelineInner {
  fn from(c: SimpleCommand) -> Self {
    PipelineInner::Command(c.into())
  }
}

impl From<Command> for PipelineInner {
  fn from(c: Command) -> Self {
    PipelineInner::Command(c)
  }
}

impl From<SimpleCommand> for Sequence {
  fn from(c: SimpleCommand) -> Self {
    let command: Command = c.into();
    command.into()
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnvVar {
  pub name: String,
  pub value: Word,
}

impl EnvVar {
  pub fn new(name: String, value: Word) -> Self {
    EnvVar { name, value }
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Word(Vec<WordPart>);

impl Word {
  pub fn new(parts: Vec<WordPart>) -> Self {
    Word(parts)
  }

  pub fn new_empty() -> Self {
    Word(vec![])
  }

  pub fn new_string(text: &str) -> Self {
    Word(vec![WordPart::Quoted(vec![WordPart::Text(
      text.to_string(),
    )])])
  }

  pub fn new_word(text: &str) -> Self {
    Word(vec![WordPart::Text(text.to_string())])
  }

  pub fn parts(&self) -> &Vec<WordPart> {
    &self.0
  }

  pub fn into_parts(self) -> Vec<WordPart> {
    self.0
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind", content = "value")
)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WordPart {
  /// Text in the string (ex. `some text`)
  Text(String),
  /// Variable substitution (ex. `$MY_VAR`)
  Variable(String),
  /// Command substitution (ex. `$(command)`)
  Command(SequentialList),
  /// Quoted string (ex. `"hello"` or `'test'`)
  Quoted(Vec<WordPart>),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind", content = "fd")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectFd {
  Fd(u32),
  StdoutStderr,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
  pub maybe_fd: Option<RedirectFd>,
  pub op: RedirectOp,
  pub io_file: IoFile,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind", content = "value")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoFile {
  /// Filename to redirect to/from (ex. `file.txt`` in `cmd < file.txt`)
  Word(Word),
  /// File descriptor to redirect to/from (ex. `2` in `cmd >&2`)
  Fd(u32),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
  feature = "serialization",
  serde(rename_all = "camelCase", tag = "kind", content = "value")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectOp {
  Input(RedirectOpInput),
  Output(RedirectOpOutput),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectOpInput {
  /// <
  Redirect,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectOpOutput {
  /// >
  Overwrite,
  /// >>
  Append,
}

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ShellParser;

pub fn parse(input: &str) -> Result<SequentialList> {
  let mut pairs = ShellParser::parse(Rule::FILE, input)?;

  parse_file(pairs.next().unwrap())
}

fn parse_file(pairs: Pair<Rule>) -> Result<SequentialList> {
  parse_complete_command(pairs.into_inner().next().unwrap())
}

fn parse_complete_command(pair: Pair<Rule>) -> Result<SequentialList> {
  assert!(pair.as_rule() == Rule::complete_command);
  let mut items = Vec::new();
  for command in pair.into_inner() {
    match command.as_rule() {
      Rule::list => {
        parse_list(command, &mut items)?;
      }
      Rule::EOI => {
        break;
      }
      _ => {
        return Err(anyhow::anyhow!(
          "Unexpected rule in complete_command: {:?}",
          command.as_rule()
        ));
      }
    }
  }
  Ok(SequentialList { items })
}

fn parse_list(
  pair: Pair<Rule>,
  items: &mut Vec<SequentialListItem>,
) -> Result<()> {
  for item in pair.into_inner() {
    match item.as_rule() {
      Rule::and_or => {
        let sequence = parse_and_or(item)?;
        items.push(SequentialListItem {
          is_async: false,
          sequence,
        });
      }
      Rule::separator_op => {
        if let Some(last) = items.last_mut() {
          last.is_async = item.as_str() == "&";
        }
      }
      _ => {
        return Err(anyhow::anyhow!(
          "Unexpected rule in list: {:?}",
          item.as_rule()
        ));
      }
    }
  }
  Ok(())
}

fn parse_compound_list(
  pair: Pair<Rule>,
  items: &mut Vec<SequentialListItem>,
) -> Result<()> {
  for item in pair.into_inner() {
    match item.as_rule() {
      Rule::term => {
        parse_term(item, items)?;
      }
      Rule::newline_list => {
        // Ignore newlines
      }
      _ => {
        return Err(anyhow::anyhow!(
          "Unexpected rule in compound_list: {:?}",
          item.as_rule()
        ));
      }
    }
  }
  Ok(())
}

fn parse_term(
  pair: Pair<Rule>,
  items: &mut Vec<SequentialListItem>,
) -> Result<()> {
  for item in pair.into_inner() {
    match item.as_rule() {
      Rule::and_or => {
        let sequence = parse_and_or(item)?;
        items.push(SequentialListItem {
          sequence,
          is_async: false,
        });
      }
      Rule::separator_op => {
        if let Some(last) = items.last_mut() {
          last.is_async = item.as_str() == "&";
        }
      }
      _ => {
        return Err(anyhow::anyhow!(
          "Unexpected rule in term: {:?}",
          item.as_rule()
        ));
      }
    }
  }
  Ok(())
}

fn parse_and_or(pair: Pair<Rule>) -> Result<Sequence> {
  assert!(pair.as_rule() == Rule::and_or);
  let mut items = pair.into_inner();
  let first_item = items.next().unwrap();
  let mut current = match first_item.as_rule() {
    Rule::ASSIGNMENT_WORD => parse_shell_var(first_item)?,
    Rule::pipeline => parse_pipeline(first_item)?,
    _ => unreachable!(),
  };

  match items.next() {
    Some(next_item) => {
      if next_item.as_rule() == Rule::ASSIGNMENT_WORD {
        anyhow::bail!(
          "Multiple assignment words before && or || is not supported yet"
        );
      } else {
        let op = match next_item.as_str() {
          "&&" => BooleanListOperator::And,
          "||" => BooleanListOperator::Or,
          _ => unreachable!(),
        };

        let next_item = items.next().unwrap();
        let next = parse_and_or(next_item)?;
        current =
          Sequence::BooleanList(Box::new(BooleanList { current, op, next }));
      }
    }
    None => {
      return Ok(current);
    }
  }

  Ok(current)
}

fn parse_shell_var(pair: Pair<Rule>) -> Result<Sequence> {
  let mut inner = pair.into_inner();
  let name = inner
    .next()
    .ok_or_else(|| anyhow::anyhow!("Expected variable name"))?
    .as_str()
    .to_string();
  let value = inner
    .next()
    .ok_or_else(|| anyhow::anyhow!("Expected variable value"))?;
  let value = parse_word(value)?;
  Ok(Sequence::ShellVar(EnvVar { name, value }))
}

fn parse_pipeline(pair: Pair<Rule>) -> Result<Sequence> {
  let mut inner = pair.into_inner();

  // Check if the first element is Bang (negation)
  let first = inner
    .next()
    .ok_or_else(|| anyhow::anyhow!("Expected pipeline content"))?;
  let (negated, pipe_sequence) = if first.as_rule() == Rule::Bang {
    // If it's Bang, the next element should be the pipe_sequence
    let pipe_sequence = inner.next().ok_or_else(|| {
      anyhow::anyhow!("Expected pipe sequence after negation")
    })?;
    (true, pipe_sequence)
  } else {
    // If it's not Bang, this element itself is the pipe_sequence
    (false, first)
  };

  let pipeline_inner = parse_pipe_sequence(pipe_sequence)?;

  Ok(Sequence::Pipeline(Pipeline {
    negated,
    inner: pipeline_inner,
  }))
}

fn parse_pipe_sequence(pair: Pair<Rule>) -> Result<PipelineInner> {
  let mut inner = pair.into_inner();

  // Parse the first command
  let first_command = inner.next().ok_or_else(|| {
    anyhow::anyhow!("Expected at least one command in pipe sequence")
  })?;
  let current = parse_command(first_command)?;

  // Check if there's a pipe operator
  match inner.next() {
    Some(pipe_op) => {
      let op = match pipe_op.as_rule() {
        Rule::Stdout => PipeSequenceOperator::Stdout,
        Rule::StdoutStderr => PipeSequenceOperator::StdoutStderr,
        _ => {
          return Err(anyhow::anyhow!(
            "Expected pipe operator, found {:?}",
            pipe_op.as_rule()
          ))
        }
      };

      // Parse the rest of the pipe sequence
      let next_sequence = inner.next().ok_or_else(|| {
        anyhow::anyhow!("Expected command after pipe operator")
      })?;
      let next = parse_pipe_sequence(next_sequence)?;

      Ok(PipelineInner::PipeSequence(Box::new(PipeSequence {
        current,
        op,
        next,
      })))
    }
    None => Ok(PipelineInner::Command(current)),
  }
}

fn parse_command(pair: Pair<Rule>) -> Result<Command> {
  let inner = pair.into_inner().next().unwrap();
  match inner.as_rule() {
    Rule::simple_command => parse_simple_command(inner),
    Rule::compound_command => parse_compound_command(inner),
    Rule::function_definition => {
      todo!("function definitions are not supported yet")
    }
    _ => Err(anyhow::anyhow!(
      "Unexpected rule in command: {:?}",
      inner.as_rule()
    )),
  }
}

fn parse_simple_command(pair: Pair<Rule>) -> Result<Command> {
  let mut env_vars = Vec::new();
  let mut args = Vec::new();
  let mut redirect = None;

  for item in pair.into_inner() {
    match item.as_rule() {
      Rule::cmd_prefix => {
        for prefix in item.into_inner() {
          match prefix.as_rule() {
            Rule::ASSIGNMENT_WORD => env_vars.push(parse_env_var(prefix)?),
            Rule::io_redirect => todo!("io_redirect as prefix"),
            _ => {
              return Err(anyhow::anyhow!(
                "Unexpected rule in cmd_prefix: {:?}",
                prefix.as_rule()
              ))
            }
          }
        }
      }
      Rule::cmd_word | Rule::cmd_name => {
        args.push(parse_word(item.into_inner().next().unwrap())?)
      }
      Rule::cmd_suffix => {
        for suffix in item.into_inner() {
          match suffix.as_rule() {
            Rule::UNQUOTED_PENDING_WORD => args.push(parse_word(suffix)?),
            Rule::io_redirect => {
              redirect = Some(parse_io_redirect(suffix)?);
            }
            Rule::QUOTED_WORD => {
              args.push(Word::new(vec![parse_quoted_word(suffix)?]))
            }
            _ => {
              return Err(anyhow::anyhow!(
                "Unexpected rule in cmd_suffix: {:?}",
                suffix.as_rule()
              ))
            }
          }
        }
      }
      _ => {
        return Err(anyhow::anyhow!(
          "Unexpected rule in simple_command: {:?}",
          item.as_rule()
        ))
      }
    }
  }

  Ok(Command {
    inner: CommandInner::Simple(SimpleCommand { env_vars, args }),
    redirect,
  })
}

fn parse_compound_command(pair: Pair<Rule>) -> Result<Command> {
  let inner = pair.into_inner().next().unwrap();
  match inner.as_rule() {
    Rule::brace_group => todo!("brace_group"),
    Rule::subshell => parse_subshell(inner),
    Rule::for_clause => todo!("for_clause"),
    Rule::case_clause => todo!("case_clause"),
    Rule::if_clause => todo!("if_clause"),
    Rule::while_clause => todo!("while_clause"),
    Rule::until_clause => todo!("until_clause"),
    _ => Err(anyhow::anyhow!(
      "Unexpected rule in compound_command: {:?}",
      inner.as_rule()
    )),
  }
}

fn parse_subshell(pair: Pair<Rule>) -> Result<Command> {
  let mut items = Vec::new();
  if let Some(inner) = pair.into_inner().next() {
    parse_compound_list(inner, &mut items)?;
    Ok(Command {
      inner: CommandInner::Subshell(Box::new(SequentialList { items })),
      redirect: None,
    })
  } else {
    Err(anyhow::anyhow!("Unexpected end of input in subshell"))
  }
}

fn parse_word(pair: Pair<Rule>) -> Result<Word> {
  let mut parts = Vec::new();

  match pair.as_rule() {
    Rule::UNQUOTED_PENDING_WORD => {
      for part in pair.into_inner() {
        match part.as_rule() {
          Rule::EXIT_STATUS => parts.push(WordPart::Variable("?".to_string())),
          Rule::UNQUOTED_ESCAPE_CHAR | Rule::UNQUOTED_CHAR => {
            if let Some(WordPart::Text(ref mut text)) = parts.last_mut() {
              text.push(part.as_str().chars().next().unwrap());
            } else {
              parts.push(WordPart::Text(part.as_str().to_string()));
            }
          }
          Rule::SUB_COMMAND => {
            let command =
              parse_complete_command(part.into_inner().next().unwrap())?;
            parts.push(WordPart::Command(command));
          }
          Rule::VARIABLE => {
            parts.push(WordPart::Variable(part.as_str().to_string()))
          }
          Rule::QUOTED_WORD => {
            let quoted = parse_quoted_word(part)?;
            parts.push(quoted);
          }
          _ => {
            return Err(anyhow::anyhow!(
              "Unexpected rule in UNQUOTED_PENDING_WORD: {:?}",
              part.as_rule()
            ))
          }
        }
      }
    }
    Rule::QUOTED_WORD => {
      let quoted = parse_quoted_word(pair)?;
      parts.push(quoted);
    }
    Rule::ASSIGNMENT_WORD => {
      let assignment_str = pair.as_str().to_string();
      parts.push(WordPart::Text(assignment_str));
    }
    Rule::FILE_NAME_PENDING_WORD => {
      for part in pair.into_inner() {
        match part.as_rule() {
          Rule::UNQUOTED_ESCAPE_CHAR => {
            if let Some(WordPart::Text(ref mut text)) = parts.last_mut() {
              text.push(part.as_str().chars().next().unwrap());
            } else {
              parts.push(WordPart::Text(part.as_str().to_string()));
            }
          }
          Rule::VARIABLE => {
            parts.push(WordPart::Variable(part.as_str().to_string()))
          }
          Rule::UNQUOTED_CHAR => {
            if let Some(WordPart::Text(ref mut text)) = parts.last_mut() {
              text.push(part.as_str().chars().next().unwrap());
            } else {
              parts.push(WordPart::Text(part.as_str().to_string()));
            }
          }
          Rule::QUOTED_WORD => {
            let quoted = parse_quoted_word(part)?;
            parts.push(quoted);
          }
          _ => {
            return Err(anyhow::anyhow!(
              "Unexpected rule in FILE_NAME_PENDING_WORD: {:?}",
              part.as_rule()
            ))
          }
        }
      }
    }
    _ => {
      return Err(anyhow::anyhow!(
        "Unexpected rule in word: {:?}",
        pair.as_rule()
      ))
    }
  }

  if parts.is_empty() {
    Ok(Word::new_empty())
  } else {
    Ok(Word::new(parts))
  }
}

fn parse_quoted_word(pair: Pair<Rule>) -> Result<WordPart> {
  let mut parts = Vec::new();
  let inner = pair.into_inner().next().unwrap();

  match inner.as_rule() {
    Rule::DOUBLE_QUOTED => {
      let inner = inner.into_inner().next().unwrap();
      for part in inner.into_inner() {
        match part.as_rule() {
          Rule::EXIT_STATUS => parts.push(WordPart::Text("$?".to_string())),
          Rule::QUOTED_ESCAPE_CHAR => {
            if let Some(WordPart::Text(ref mut s)) = parts.last_mut() {
              s.push_str(part.as_str());
            } else {
              parts.push(WordPart::Text(part.as_str().to_string()));
            }
          }
          Rule::SUB_COMMAND => {
            let command =
              parse_complete_command(part.into_inner().next().unwrap())?;
            parts.push(WordPart::Command(command));
          }
          Rule::VARIABLE => {
            parts.push(WordPart::Variable(part.as_str()[1..].to_string()))
          }
          Rule::QUOTED_CHAR => {
            if let Some(WordPart::Text(ref mut s)) = parts.last_mut() {
              s.push_str(part.as_str());
            } else {
              parts.push(WordPart::Text(part.as_str().to_string()));
            }
          }
          _ => {
            return Err(anyhow::anyhow!(
              "Unexpected rule in DOUBLE_QUOTED: {:?}",
              part.as_rule()
            ))
          }
        }
      }
      Ok(WordPart::Quoted(parts))
    }
    Rule::SINGLE_QUOTED => {
      let inner_str = inner.as_str();
      let trimmed_str = &inner_str[1..inner_str.len() - 1];
      Ok(WordPart::Quoted(vec![WordPart::Text(
        trimmed_str.to_string(),
      )]))
    }
    _ => Err(anyhow::anyhow!(
      "Unexpected rule in QUOTED_WORD: {:?}",
      inner.as_rule()
    )),
  }
}

fn parse_env_var(pair: Pair<Rule>) -> Result<EnvVar> {
  let mut parts = pair.into_inner();

  // Get the name of the environment variable
  let name = parts
    .next()
    .ok_or_else(|| anyhow!("Expected variable name"))?
    .as_str()
    .to_string();

  // Get the value of the environment variable
  let word_value = if let Some(value) = parts.next() {
    parse_word(value)?
  } else {
    Word::new_empty()
  };

  Ok(EnvVar {
    name,
    value: word_value,
  })
}

fn parse_io_redirect(pair: Pair<Rule>) -> Result<Redirect> {
  let mut inner = pair.into_inner();

  // Parse the optional IO number or AMPERSAND
  let (maybe_fd, op_and_file) = match inner.next() {
    Some(p) if p.as_rule() == Rule::IO_NUMBER => (
      Some(RedirectFd::Fd(p.as_str().parse::<u32>().unwrap())),
      inner.next().ok_or_else(|| {
        anyhow!("Expected redirection operator after IO number")
      })?,
    ),
    Some(p) if p.as_rule() == Rule::AMPERSAND => (
      Some(RedirectFd::StdoutStderr),
      inner
        .next()
        .ok_or_else(|| anyhow!("Expected redirection operator after &"))?,
    ),
    Some(p) => (None, p),
    None => return Err(anyhow!("Unexpected end of input in io_redirect")),
  };

  let (op, io_file) = parse_io_file(op_and_file)?;

  Ok(Redirect {
    maybe_fd,
    op,
    io_file,
  })
}

fn parse_io_file(pair: Pair<Rule>) -> Result<(RedirectOp, IoFile)> {
  let mut inner = pair.into_inner();
  let op = inner
    .next()
    .ok_or_else(|| anyhow!("Expected redirection operator"))?;
  let filename = inner
    .next()
    .ok_or_else(|| anyhow!("Expected filename after redirection operator"))?;

  let redirect_op = match op.as_rule() {
    Rule::LESS => RedirectOp::Input(RedirectOpInput::Redirect),
    Rule::GREAT => RedirectOp::Output(RedirectOpOutput::Overwrite),
    Rule::DGREAT => RedirectOp::Output(RedirectOpOutput::Append),
    Rule::LESSAND | Rule::GREATAND => {
      // For these operators, the target must be a number (fd)
      let target = filename.as_str();
      if let Ok(fd) = target.parse::<u32>() {
        return Ok((
          if op.as_rule() == Rule::LESSAND {
            RedirectOp::Input(RedirectOpInput::Redirect)
          } else {
            RedirectOp::Output(RedirectOpOutput::Overwrite)
          },
          IoFile::Fd(fd),
        ));
      } else {
        return Err(anyhow!(
          "Expected a number after {} operator",
          if op.as_rule() == Rule::LESSAND {
            "<&"
          } else {
            ">&"
          }
        ));
      }
    }
    _ => {
      return Err(anyhow!(
        "Unexpected redirection operator: {:?}",
        op.as_rule()
      ))
    }
  };

  let io_file = if filename.as_rule() == Rule::FILE_NAME_PENDING_WORD {
    IoFile::Word(parse_word(filename)?)
  } else {
    return Err(anyhow!(
      "Unexpected filename type: {:?}",
      filename.as_rule()
    ));
  };

  Ok((redirect_op, io_file))
}

#[cfg(test)]
mod test {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_main() {
    assert!(parse("&& testing").is_err());
    assert!(parse("test { test").is_err());
    assert!(parse("cp test/* other").is_ok());
    assert!(parse("cp test/? other").is_ok());
    assert!(parse("(test").is_err());
    assert!(parse("cmd \"test").is_err());
    assert!(parse("cmd 'test").is_err());

    assert!(parse("( test ||other&&test;test);(t&est );").is_ok());
    assert!(parse("command --arg='value'").is_ok());
    assert!(parse("command --arg=\"value\"").is_ok());

    assert!(
      parse("deno run --allow-read=. --allow-write=./testing main.ts").is_ok(),
    );

    assert!(parse("echo \"foo\" > out.txt").is_ok());
  }

  #[test]
  fn test_sequential_list() {
    let parse_and_create = |input: &str| -> Result<SequentialList> {
      let pairs = ShellParser::parse(Rule::complete_command, input)
        .map_err(|e| anyhow::Error::msg(e.to_string()))?
        .next()
        .unwrap();
      //   println!("pairs: {:?}", pairs);
      parse_complete_command(pairs)
    };

    // Test case 1
    let input = concat!(
      "Name=Value OtherVar=Other command arg1 || command2 arg12 arg13 ; ",
      "command3 && command4 & command5 ; export ENV6=5 ; ",
      "ENV7=other && command8 || command9 ; ",
      "cmd10 && (cmd11 || cmd12)"
    );
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![
        SequentialListItem {
          is_async: false,
          sequence: Sequence::BooleanList(Box::new(BooleanList {
            current: SimpleCommand {
              env_vars: vec![
                EnvVar::new("Name".to_string(), Word::new_word("Value")),
                EnvVar::new("OtherVar".to_string(), Word::new_word("Other")),
              ],
              args: vec![Word::new_word("command"), Word::new_word("arg1")],
            }
            .into(),
            op: BooleanListOperator::Or,
            next: SimpleCommand {
              env_vars: vec![],
              args: vec![
                Word::new_word("command2"),
                Word::new_word("arg12"),
                Word::new_word("arg13"),
              ],
            }
            .into(),
          })),
        },
        SequentialListItem {
          is_async: true,
          sequence: Sequence::BooleanList(Box::new(BooleanList {
            current: SimpleCommand {
              env_vars: vec![],
              args: vec![Word::new_word("command3")],
            }
            .into(),
            op: BooleanListOperator::And,
            next: SimpleCommand {
              env_vars: vec![],
              args: vec![Word::new_word("command4")],
            }
            .into(),
          })),
        },
        SequentialListItem {
          is_async: false,
          sequence: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("command5")],
          }
          .into(),
        },
        SequentialListItem {
          is_async: false,
          sequence: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("export"), Word::new_word("ENV6=5")],
          }
          .into(),
        },
        SequentialListItem {
          is_async: false,
          sequence: Sequence::BooleanList(Box::new(BooleanList {
            current: Sequence::ShellVar(EnvVar::new(
              "ENV7".to_string(),
              Word::new_word("other"),
            )),
            op: BooleanListOperator::And,
            next: Sequence::BooleanList(Box::new(BooleanList {
              current: SimpleCommand {
                env_vars: vec![],
                args: vec![Word::new_word("command8")],
              }
              .into(),
              op: BooleanListOperator::Or,
              next: SimpleCommand {
                env_vars: vec![],
                args: vec![Word::new_word("command9")],
              }
              .into(),
            })),
          })),
        },
        SequentialListItem {
          is_async: false,
          sequence: Sequence::BooleanList(Box::new(BooleanList {
            current: SimpleCommand {
              env_vars: vec![],
              args: vec![Word::new_word("cmd10")],
            }
            .into(),
            op: BooleanListOperator::And,
            next: Command {
              inner: CommandInner::Subshell(Box::new(SequentialList {
                items: vec![SequentialListItem {
                  is_async: false,
                  sequence: Sequence::BooleanList(Box::new(BooleanList {
                    current: SimpleCommand {
                      env_vars: vec![],
                      args: vec![Word::new_word("cmd11")],
                    }
                    .into(),
                    op: BooleanListOperator::Or,
                    next: SimpleCommand {
                      env_vars: vec![],
                      args: vec![Word::new_word("cmd12")],
                    }
                    .into(),
                  })),
                }],
              })),
              redirect: None,
            }
            .into(),
          })),
        },
      ],
    };
    assert_eq!(result, expected);

    // Test case 2
    let input = "command1 ; command2 ; A='b' command3";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![
        SequentialListItem {
          is_async: false,
          sequence: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("command1")],
          }
          .into(),
        },
        SequentialListItem {
          is_async: false,
          sequence: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("command2")],
          }
          .into(),
        },
        SequentialListItem {
          is_async: false,
          sequence: SimpleCommand {
            env_vars: vec![EnvVar::new("A".to_string(), Word::new_string("b"))],
            args: vec![Word::new_word("command3")],
          }
          .into(),
        },
      ],
    };
    assert_eq!(result, expected);

    // Test case 3
    let input = "test &&";
    assert!(parse_and_create(input).is_err());

    // Test case 4
    let input = "command &";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![SequentialListItem {
        is_async: true,
        sequence: SimpleCommand {
          env_vars: vec![],
          args: vec![Word::new_word("command")],
        }
        .into(),
      }],
    };
    assert_eq!(result, expected);

    // Test case 5
    let input = "test | other";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![SequentialListItem {
        is_async: false,
        sequence: PipeSequence {
          current: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("test")],
          }
          .into(),
          op: PipeSequenceOperator::Stdout,
          next: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("other")],
          }
          .into(),
        }
        .into(),
      }],
    };
    assert_eq!(result, expected);

    // Test case 6
    let input = "test |& other";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![SequentialListItem {
        is_async: false,
        sequence: PipeSequence {
          current: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("test")],
          }
          .into(),
          op: PipeSequenceOperator::StdoutStderr,
          next: SimpleCommand {
            env_vars: vec![],
            args: vec![Word::new_word("other")],
          }
          .into(),
        }
        .into(),
      }],
    };
    assert_eq!(result, expected);

    // Test case 8
    let input = "echo $MY_ENV;";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![SequentialListItem {
        is_async: false,
        sequence: SimpleCommand {
          env_vars: vec![],
          args: vec![
            Word::new_word("echo"),
            Word(vec![WordPart::Variable("MY_ENV".to_string())]),
          ],
        }
        .into(),
      }],
    };
    assert_eq!(result, expected);

    // Test case 9
    let input = "! cmd1 | cmd2 && cmd3";
    let result = parse_and_create(input).unwrap();
    let expected = SequentialList {
      items: vec![SequentialListItem {
        is_async: false,
        sequence: Sequence::BooleanList(Box::new(BooleanList {
          current: Pipeline {
            negated: true,
            inner: PipeSequence {
              current: SimpleCommand {
                args: vec![Word::new_word("cmd1")],
                env_vars: vec![],
              }
              .into(),
              op: PipeSequenceOperator::Stdout,
              next: SimpleCommand {
                args: vec![Word::new_word("cmd2")],
                env_vars: vec![],
              }
              .into(),
            }
            .into(),
          }
          .into(),
          op: BooleanListOperator::And,
          next: SimpleCommand {
            args: vec![Word::new_word("cmd3")],
            env_vars: vec![],
          }
          .into(),
        })),
      }],
    };
    assert_eq!(result, expected);
  }

  #[test]
  fn test_env_var() {
    let parse_and_create = |input: &str| -> Result<EnvVar, anyhow::Error> {
      let pairs = ShellParser::parse(Rule::ASSIGNMENT_WORD, input)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .next()
        .unwrap();
      parse_env_var(pairs)
    };

    assert_eq!(
      parse_and_create("Name=Value").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word::new_word("Value"),
      }
    );

    assert_eq!(
      parse_and_create("Name='quoted value'").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word::new_string("quoted value"),
      }
    );

    assert_eq!(
      parse_and_create("Name=\"double quoted value\"").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word::new_string("double quoted value"),
      }
    );

    assert_eq!(
      parse_and_create("Name=").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word(vec![]),
      }
    );

    assert_eq!(
      parse_and_create("Name=$(test)").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word(vec![WordPart::Command(SequentialList {
          items: vec![SequentialListItem {
            is_async: false,
            sequence: SimpleCommand {
              env_vars: vec![],
              args: vec![Word::new_word("test")],
            }
            .into(),
          }],
        })]),
      }
    );

    assert_eq!(
      parse_and_create("Name=$(OTHER=5)").unwrap(),
      EnvVar {
        name: "Name".to_string(),
        value: Word(vec![WordPart::Command(SequentialList {
          items: vec![SequentialListItem {
            is_async: false,
            sequence: Sequence::ShellVar(EnvVar {
              name: "OTHER".to_string(),
              value: Word::new_word("5"),
            }),
          }],
        })]),
      }
    );
  }

  #[cfg(feature = "serialization")]
  #[test]
  fn serializes_command_to_json() {
    assert_json_equals(
      serialize_to_json("./example > output.txt"),
      serde_json::json!({
        "items": [{
          "isAsync": false,
          "sequence": {
            "inner": {
              "inner": {
                "args": [[{
                  "kind": "text",
                  "value": "./example"
                }]],
                "envVars": [],
                "kind": "simple"
              },
              "kind": "command",
              "redirect": {
                "ioFile": {
                  "kind": "word",
                  "value": [{
                    "kind": "text",
                    "value": "output.txt"
                  }],
                },
                "maybeFd": null,
                "op": {
                  "kind": "output",
                  "value": "overwrite",
                }
              }
            },
            "kind": "pipeline",
            "negated": false
          }
        }]
      }),
    );
    assert_json_equals(
      serialize_to_json("./example 2> output.txt"),
      serde_json::json!({
        "items": [{
          "isAsync": false,
          "sequence": {
            "inner": {
              "inner": {
                "args": [[{
                  "kind": "text",
                  "value": "./example"
                }]],
                "envVars": [],
                "kind": "simple"
              },
              "kind": "command",
              "redirect": {
                "ioFile": {
                  "kind": "word",
                  "value": [{
                    "kind": "text",
                    "value": "output.txt"
                  }],
                },
                "maybeFd": {
                  "kind": "fd",
                  "fd": 2,
                },
                "op": {
                  "kind": "output",
                  "value": "overwrite",
                }
              }
            },
            "kind": "pipeline",
            "negated": false
          }
        }]
      }),
    );
    assert_json_equals(
      serialize_to_json("./example &> output.txt"),
      serde_json::json!({
        "items": [{
          "isAsync": false,
          "sequence": {
            "inner": {
              "inner": {
                "args": [[{
                  "kind": "text",
                  "value": "./example"
                }]],
                "envVars": [],
                "kind": "simple"
              },
              "kind": "command",
              "redirect": {
                "ioFile": {
                  "kind": "word",
                  "value": [{
                    "kind": "text",
                    "value": "output.txt"
                  }],
                },
                "maybeFd": {
                  "kind": "stdoutStderr"
                },
                "op": {
                  "kind": "output",
                  "value": "overwrite",
                }
              }
            },
            "kind": "pipeline",
            "negated": false
          }
        }]
      }),
    );
    assert_json_equals(
      serialize_to_json("./example < output.txt"),
      serde_json::json!({
        "items": [{
          "isAsync": false,
          "sequence": {
            "inner": {
              "inner": {
                "args": [[{
                  "kind": "text",
                  "value": "./example"
                }]],
                "envVars": [],
                "kind": "simple"
              },
              "kind": "command",
              "redirect": {
                "ioFile": {
                  "kind": "word",
                  "value": [{
                    "kind": "text",
                    "value": "output.txt"
                  }],
                },
                "maybeFd": null,
                "op": {
                  "kind": "input",
                  "value": "redirect",
                }
              }
            },
            "kind": "pipeline",
            "negated": false
          }
        }]
      }),
    );

    assert_json_equals(
      serialize_to_json("./example <&0"),
      serde_json::json!({
        "items": [{
          "isAsync": false,
          "sequence": {
            "inner": {
              "inner": {
                "args": [[{
                  "kind": "text",
                  "value": "./example"
                }]],
                "envVars": [],
                "kind": "simple"
              },
              "kind": "command",
              "redirect": {
                "ioFile": {
                  "kind": "fd",
                  "value": 0,
                },
                "maybeFd": null,
                "op": {
                  "kind": "input",
                  "value": "redirect",
                }
              }
            },
            "kind": "pipeline",
            "negated": false
          }
        }]
      }),
    );
  }

  #[cfg(feature = "serialization")]
  #[track_caller]
  fn assert_json_equals(
    actual: serde_json::Value,
    expected: serde_json::Value,
  ) {
    if actual != expected {
      let actual = serde_json::to_string_pretty(&actual).unwrap();
      let expected = serde_json::to_string_pretty(&expected).unwrap();
      assert_eq!(actual, expected);
    }
  }

  #[cfg(feature = "serialization")]
  fn serialize_to_json(text: &str) -> serde_json::Value {
    let command = parse(text).unwrap();
    serde_json::to_value(command).unwrap()
  }
}
