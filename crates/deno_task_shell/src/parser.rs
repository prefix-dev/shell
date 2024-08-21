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
    let pairs = ShellParser::parse(Rule::FILE, input)?;
    parse_complete_command(pairs.into_iter().next().unwrap())
}

fn parse_complete_command(pair: Pair<Rule>) -> Result<SequentialList> {
    let mut items = Vec::new();
    for list in pair.into_inner() {
        if list.as_rule() == Rule::list {
            let mut is_async = false;
            for item in list.into_inner() {
                match item.as_rule() {
                    Rule::and_or => {
                        let result = parse_and_or(item);
                        match result {
                            Ok(sequence) => {
                                items.push(SequentialListItem {
                                    sequence,
                                    is_async,
                                });
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Rule::separator_op => {
                        is_async = item.as_str() == "&";
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unexpected rule: {}",
                            item.as_str()
                        ));
                    }
                }
            }
        }
    }
    Ok(SequentialList { items })
}

fn parse_and_or(pair: Pair<Rule>) -> Result<Sequence> {
    let mut pipelines = pair.into_inner();
    let first_pipeline = pipelines.next().unwrap();
    let mut current = parse_pipeline(first_pipeline).unwrap();

    while let Some(op) = pipelines.next() {
        if let Some(next_pipeline) = pipelines.next() {
            let op = match op.as_str() {
                "&&" => BooleanListOperator::And,
                "||" => BooleanListOperator::Or,
                _ => unreachable!(),
            };
            let next = parse_pipeline(next_pipeline)?;
            current = Sequence::BooleanList(Box::new(BooleanList {
                current,
                op,
                next,
            }));
        }
    }

    Ok(current)
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
        Rule::compound_command => todo!("inner"),
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
                        Rule::ASSIGNMENT_WORD => {
                            env_vars.push(parse_env_var(prefix)?)
                        }
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
                        Rule::WORD => args.push(parse_word(suffix)?),
                        Rule::io_redirect => {
                            redirect = Some(parse_io_redirect(suffix)?);
                        }
                        Rule::QUOTED_WORD => args.push(parse_word(suffix)?),
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

fn parse_word(pair: Pair<Rule>) -> Result<Word> {
    let mut parts = Vec::new();

    match pair.as_rule() {
        Rule::WORD => {
            let text = pair.as_str();
            if text.starts_with('\'') && text.ends_with('\'') {
                // Single quoted text
                parts.push(WordPart::Quoted(vec![WordPart::Text(
                    text[1..text.len() - 1].to_string(),
                )]));
            } else if text.starts_with('"') && text.ends_with('"') {
                // Double quoted text
                parts.push(WordPart::Quoted(vec![WordPart::Text(
                    text[1..text.len() - 1].to_string(),
                )]));
            } else if let Some(var_name) = text.strip_prefix('$') {
                // Variable
                parts.push(WordPart::Variable(var_name.to_string()));
            } else {
                // Regular text
                parts.push(WordPart::Text(text.to_string()));
            }
        }
        Rule::WORD_WITH_EQUAL => {
            // Handle words that might start with '='
            let text = pair.as_str();
            if let Some(rest) = text.strip_prefix('=') {
                parts.push(WordPart::Text("=".to_string()));
                parts.push(WordPart::Text(rest.to_string()));
            } else if text.starts_with('\'') && text.ends_with('\'') {
                // Single quoted text
                parts.push(WordPart::Quoted(vec![WordPart::Text(
                    text[1..text.len() - 1].to_string(),
                )]));
            } else if text.starts_with('"') && text.ends_with('"') {
                // Double quoted text
                parts.push(WordPart::Quoted(vec![WordPart::Text(
                    text[1..text.len() - 1].to_string(),
                )]));
            } else if let Some(var_name) = text.strip_prefix('$') {
                // Variable
                parts.push(WordPart::Variable(var_name.to_string()));
            } else {
                // Regular text
                parts.push(WordPart::Text(text.to_string()));
            }
        }
        Rule::QUOTED_WORD => {
            let text = pair.as_str();
            let unquoted_text = &text[1..text.len() - 1];
            parts.push(WordPart::Quoted(vec![WordPart::Text(
                unquoted_text.to_string(),
            )]));
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

fn parse_env_var(pair: Pair<Rule>) -> Result<EnvVar> {
    let mut parts = pair.into_inner();

    // Get the name of the environment variable
    let name = parts
        .next()
        .ok_or_else(|| anyhow!("Expected variable name"))?
        .as_str()
        .to_string();

    // Get the value of the environment variable
    let value = parts
        .next()
        .ok_or_else(|| anyhow!("Expected variable value"))?;

    // Parse the value as a Word
    let word_value = parse_word(value)?;

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
            inner.next().ok_or_else(|| {
                anyhow!("Expected redirection operator after &")
            })?,
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
    let filename = inner.next().ok_or_else(|| {
        anyhow!("Expected filename after redirection operator")
    })?;

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

    let io_file = if filename.as_rule() == Rule::WORD {
        IoFile::Word(parse_word(filename)?)
    } else {
        return Err(anyhow!(
            "Unexpected filename type: {:?}",
            filename.as_rule()
        ));
    };

    Ok((redirect_op, io_file))
}
