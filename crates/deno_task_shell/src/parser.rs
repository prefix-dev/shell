// Copyright 2018-2024 the Deno authors. MIT license.

use lazy_static::lazy_static;
use miette::{miette, Context, Result};
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
use thiserror::Error;

// Shell grammar rules this is loosely based on:
// https://pubs.opengroup.org/onlinepubs/009604499/utilities/xcu_chap02.html#tag_02_10_02

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid sequential list")]
pub struct SequentialList {
    pub items: Vec<SequentialListItem>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid sequential list item")]
pub struct SequentialListItem {
    pub is_async: bool,
    pub sequence: Sequence,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serialization",
    serde(rename_all = "camelCase", tag = "kind")
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Sequence {
    #[error("Invalid shell variable")]
    ShellVar(EnvVar),
    #[error("Invalid pipeline")]
    Pipeline(Pipeline),
    #[error("Invalid boolean list")]
    BooleanList(Box<BooleanList>),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid pipeline")]
pub struct Pipeline {
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PipelineInner {
    #[error("Invalid command")]
    Command(Command),
    #[error("Invalid pipe sequence")]
    PipeSequence(Box<PipeSequence>),
}

impl From<PipeSequence> for PipelineInner {
    fn from(p: PipeSequence) -> Self {
        PipelineInner::PipeSequence(Box::new(p))
    }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum BooleanListOperator {
    #[error("AND operator")]
    And,
    #[error("OR operator")]
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid boolean list")]
pub struct BooleanList {
    pub current: Sequence,
    pub op: BooleanListOperator,
    pub next: Sequence,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid pipe sequence")]
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
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum PipeSequenceOperator {
    #[error("Stdout pipe operator")]
    Stdout,
    #[error("Stdout and stderr pipe operator")]
    StdoutStderr,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid command")]
pub struct Command {
    pub inner: CommandInner,
    pub redirect: Option<Redirect>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serialization",
    serde(rename_all = "camelCase", tag = "kind")
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommandInner {
    #[error("Invalid simple command")]
    Simple(SimpleCommand),
    #[error("Invalid subshell")]
    Subshell(Box<SequentialList>),
    #[error("Invalid if command")]
    If(IfClause),
    #[error("Invalid for loop")]
    For(ForLoop),
    #[error("Invalid arithmetic expression")]
    ArithmeticExpression(Arithmetic),
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid simple command")]
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
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid if clause")]
pub struct IfClause {
    pub condition: Condition,
    pub then_body: SequentialList,
    pub else_part: Option<ElsePart>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid for loop")]
pub struct ForLoop {
    pub var_name: String,
    pub wordlist: Vec<Word>,
    pub body: SequentialList,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid else part")]
pub enum ElsePart {
    Elif(Box<IfClause>),
    Else(SequentialList),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid condition")]
pub struct Condition {
    pub condition_inner: ConditionInner,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid condition inner")]
pub enum ConditionInner {
    Binary {
        left: Word,
        op: BinaryOp,
        right: Word,
    },
    Unary {
        op: Option<UnaryOp>,
        right: Word,
    },
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid binary operator")]
pub enum BinaryOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid unary operator")]
pub enum UnaryOp {
    FileExists,
    BlockSpecial,
    CharSpecial,
    Directory,
    RegularFile,
    SetGroupId,
    SymbolicLink,
    StickyBit,
    NamedPipe,
    Readable,
    SizeNonZero,
    TerminalFd,
    SetUserId,
    Writable,
    Executable,
    OwnedByEffectiveGroupId,
    ModifiedSinceLastRead,
    OwnedByEffectiveUserId,
    Socket,
    NonEmptyString,
    EmptyString,
    VariableSet,
    VariableNameReference,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid environment variable")]
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
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid tilde prefix")]
pub struct TildePrefix {
    pub user: Option<String>,
}

impl TildePrefix {
    pub fn only_tilde(self) -> bool {
        self.user.is_none()
    }

    pub fn new(user: Option<String>) -> Self {
        TildePrefix { user }
    }
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid word")]
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
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("Invalid variable modifier")]
pub enum VariableModifier {
    #[error("Invalid substring")]
    Substring {
        begin: Word,
        length: Option<Word>,
    },
    DefaultValue(Word),
    AssignDefault(Word),
    AlternateValue(Word),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serialization",
    serde(rename_all = "camelCase", tag = "kind", content = "value")
)]
#[derive(Debug, PartialEq, Eq, Clone, Error)]
pub enum WordPart {
    #[error("Invalid text")]
    Text(String),
    #[error("Invalid variable")]
    Variable(String, Option<Box<VariableModifier>>),
    #[error("Invalid command")]
    Command(SequentialList),
    #[error("Invalid quoted string")]
    Quoted(Vec<WordPart>),
    #[error("Invalid tilde prefix")]
    Tilde(TildePrefix),
    #[error("Invalid arithmetic expression")]
    Arithmetic(Arithmetic),
    #[error("Invalid exit status")]
    ExitStatus,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid arithmetic sequence")]
pub struct Arithmetic {
    pub parts: Vec<ArithmeticPart>,
}
#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid arithmetic part")]
pub enum ArithmeticPart {
    #[error("Invalid parentheses expression")]
    ParenthesesExpr(Box<Arithmetic>),
    #[error("Invalid variable assignment")]
    VariableAssignment {
        name: String,
        op: AssignmentOp,
        value: Box<ArithmeticPart>,
    },
    #[error("Invalid triple conditional expression")]
    TripleConditionalExpr {
        condition: Box<ArithmeticPart>,
        true_expr: Box<ArithmeticPart>,
        false_expr: Box<ArithmeticPart>,
    },
    #[error("Invalid binary arithmetic expression")]
    BinaryArithmeticExpr {
        left: Box<ArithmeticPart>,
        operator: BinaryArithmeticOp,
        right: Box<ArithmeticPart>,
    },
    #[error("Invalid binary conditional expression")]
    BinaryConditionalExpr {
        left: Box<ArithmeticPart>,
        operator: BinaryOp,
        right: Box<ArithmeticPart>,
    },
    #[error("Invalid unary arithmetic expression")]
    UnaryArithmeticExpr {
        operator: UnaryArithmeticOp,
        operand: Box<ArithmeticPart>,
    },
    #[error("Invalid post arithmetic expression")]
    PostArithmeticExpr {
        operand: Box<ArithmeticPart>,
        operator: PostArithmeticOp,
    },
    #[error("Invalid variable")]
    Variable(String),
    #[error("Invalid number")]
    Number(String),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Copy, Ord)]
pub enum BinaryArithmeticOp {
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    Power,      // **
    LeftShift,  // <<
    RightShift, // >>
    BitwiseAnd, // &
    BitwiseXor, // ^
    BitwiseOr,  // |
    LogicalAnd, // &&
    LogicalOr,  // ||
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AssignmentOp {
    Assign,           // =
    MultiplyAssign,   // *=
    DivideAssign,     // /=
    ModuloAssign,     // %=
    AddAssign,        // +=
    SubtractAssign,   // -=
    LeftShiftAssign,  // <<=
    RightShiftAssign, // >>=
    BitwiseAndAssign, // &=
    BitwiseXorAssign, // ^=
    BitwiseOrAssign,  // |=
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum UnaryArithmeticOp {
    Plus,       // +
    Minus,      // -
    LogicalNot, // !
    BitwiseNot, // ~
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostArithmeticOp {
    Increment, // ++
    Decrement, // --
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serialization",
    serde(rename_all = "camelCase", tag = "kind", content = "fd")
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RedirectFd {
    #[error("Invalid file descriptor")]
    Fd(u32),
    #[error("Invalid stdout and stderr redirect")]
    StdoutStderr,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid redirect")]
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IoFile {
    #[error("Invalid word")]
    Word(Word),
    #[error("Invalid file descriptor")]
    Fd(u32),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serialization",
    serde(rename_all = "camelCase", tag = "kind", content = "value")
)]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RedirectOp {
    #[error("Invalid input redirect")]
    Input(RedirectOpInput),
    #[error("Invalid output redirect")]
    Output(RedirectOpOutput),
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RedirectOpInput {
    #[error("Invalid input redirect")]
    Redirect,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RedirectOpOutput {
    #[error("Invalid overwrite redirect")]
    Overwrite,
    #[error("Invalid append redirect")]
    Append,
}

lazy_static! {
    static ref ARITHMETIC_PARSER: PrattParser<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrattParser::new()
            .op(Op::infix(assign, Right)
                | Op::infix(multiply_assign, Right)
                | Op::infix(divide_assign, Right)
                | Op::infix(modulo_assign, Right)
                | Op::infix(add_assign, Right)
                | Op::infix(subtract_assign, Right)
                | Op::infix(left_shift_assign, Right)
                | Op::infix(right_shift_assign, Right)
                | Op::infix(bitwise_and_assign, Right)
                | Op::infix(bitwise_xor_assign, Right)
                | Op::infix(bitwise_or_assign, Right))
            .op(Op::infix(logical_or, Left))
            .op(Op::infix(logical_and, Left))
            .op(Op::infix(bitwise_or, Left))
            .op(Op::infix(bitwise_xor, Left))
            .op(Op::infix(bitwise_and, Left))
            .op(Op::infix(left_shift, Left) | Op::infix(right_shift, Left))
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
            .op(Op::infix(multiply, Left)
                | Op::infix(divide, Left)
                | Op::infix(modulo, Left))
            .op(Op::infix(power, Right))
    };
}

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ShellParser;

pub fn debug_parse(input: &str) {
    let parsed = ShellParser::parse(Rule::FILE, input);
    pest_ascii_tree::print_ascii_tree(parsed);
}

pub fn parse(input: &str) -> Result<SequentialList> {
    let mut pairs = ShellParser::parse(Rule::FILE, input).map_err(|e| {
        miette::Error::new(e.into_miette()).context("Failed to parse input")
    })?;

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
                return Err(miette!(
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
                return Err(miette!(
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
            Rule::separator_op => {
                if let Some(last) = items.last_mut() {
                    last.is_async = item.as_str() == "&";
                }
            }
            _ => {
                return Err(miette!(
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
                return Err(miette!(
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
                return Err(miette!(
          "Multiple assignment words before && or || is not supported yet"
        ));
            } else {
                let op = match next_item.as_str() {
                    "&&" => BooleanListOperator::And,
                    "||" => BooleanListOperator::Or,
                    _ => unreachable!(),
                };

                let next_item = items.next().unwrap();
                let next = parse_and_or(next_item)?;
                current = Sequence::BooleanList(Box::new(BooleanList {
                    current,
                    op,
                    next,
                }));
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
        .ok_or_else(|| miette!("Expected variable name"))?
        .as_str()
        .to_string();
    let value = inner
        .next()
        .ok_or_else(|| miette!("Expected variable value"))?;
    let value = parse_assignment_value(value)?;
    Ok(Sequence::ShellVar(EnvVar { name, value }))
}

fn parse_pipeline(pair: Pair<Rule>) -> Result<Sequence> {
    let pipeline_str = pair.as_str();
    let mut inner = pair.into_inner();

    // Check if the first element is Bang (negation)
    let first = inner
        .next()
        .ok_or_else(|| miette!("Expected pipeline content"))?;
    let (negated, pipe_sequence) = if first.as_rule() == Rule::Bang {
        // If it's Bang, check for whitespace
        if pipeline_str.len() > 1
            && !pipeline_str[1..2].chars().next().unwrap().is_whitespace()
        {
            return Err(miette!(
        "Perhaps you meant to add a space after the exclamation point to negate the command?\n  ! {}", 
        pipeline_str
      ));
        }
        // Get the actual pipe sequence after whitespace
        let pipe_sequence = inner
            .next()
            .ok_or_else(|| miette!("Expected pipe sequence after negation"))?;
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
        miette!("Expected at least one command in pipe sequence")
    })?;
    let current = parse_command(first_command)?;

    // Check if there's a pipe operator
    match inner.next() {
        Some(pipe_op) => {
            let op = match pipe_op.as_rule() {
                Rule::Stdout => PipeSequenceOperator::Stdout,
                Rule::StdoutStderr => PipeSequenceOperator::StdoutStderr,
                _ => {
                    return Err(miette!(
                        "Expected pipe operator, found {:?}",
                        pipe_op.as_rule()
                    ));
                }
            };

            // Parse the rest of the pipe sequence
            let next_sequence = inner.next().ok_or_else(|| {
                miette!("Expected command after pipe operator")
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
            Err(miette!("Function definitions are not supported yet"))
        }
        _ => Err(miette!("Unexpected rule in command: {:?}", inner.as_rule())),
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
                        Rule::io_redirect => {
                            return Err(miette!("io_redirect as prefix"))
                        }
                        _ => {
                            return Err(miette!(
                                "Unexpected rule in cmd_prefix: {:?}",
                                prefix.as_rule()
                            ));
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
                        Rule::UNQUOTED_PENDING_WORD => {
                            args.push(parse_word(suffix)?)
                        }
                        Rule::io_redirect => {
                            redirect = Some(parse_io_redirect(suffix)?);
                        }
                        Rule::QUOTED_WORD => args
                            .push(Word::new(vec![parse_quoted_word(suffix)?])),
                        _ => {
                            return Err(miette!(
                                "Unexpected rule in cmd_suffix: {:?}",
                                suffix.as_rule()
                            ));
                        }
                    }
                }
            }
            _ => {
                return Err(miette!(
                    "Unexpected rule in simple_command: {:?}",
                    item.as_rule()
                ));
            }
        }
    }

    Ok(Command {
        inner: CommandInner::Simple(SimpleCommand { env_vars, args }),
        redirect,
    })
}

fn parse_wordlist(pair: Pair<Rule>) -> Result<Vec<Word>> {
    let mut words = Vec::new();
    for word_pair in pair.into_inner() {
        words.push(parse_word(word_pair)?);
    }
    Ok(words)
}

fn parse_do_group(pairs: Pair<Rule>) -> Result<SequentialList> {
    let mut inner = pairs.into_inner();
    let mut result = Vec::new();
    let body_pair = inner
        .next()
        .ok_or_else(|| miette!("Expected body in do group"))?;
    parse_compound_list(body_pair, &mut result)?;

    Ok(SequentialList { items: result })
}

fn parse_for_loop(pairs: Pair<Rule>) -> Result<ForLoop> {
    let mut inner = pairs.into_inner();

    let var_name = inner
        .next()
        .ok_or_else(|| miette!("Expected variable name in for loop"))?
        .as_str()
        .to_string();

    let wordlist = match inner.next() {
        Some(wordlist_pair) => parse_wordlist(wordlist_pair)?,
        None => panic!("Expected wordlist in for loop"),
    };

    let body_pair = inner
        .next()
        .ok_or_else(|| miette!("Expected body in for loop"))?;

    let body = parse_do_group(body_pair)?;

    Ok(ForLoop {
        var_name,
        wordlist,
        body,
    })
}

fn parse_compound_command(pair: Pair<Rule>) -> Result<Command> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::brace_group => {
            Err(miette!("Unsupported compound command brace_group"))
        }
        Rule::subshell => parse_subshell(inner),
        Rule::for_clause => {
            let for_loop = parse_for_loop(inner);
            Ok(Command {
                inner: CommandInner::For(for_loop?),
                redirect: None,
            })
        }
        Rule::case_clause => {
            Err(miette!("Unsupported compound command case_clause"))
        }
        Rule::if_clause => {
            let if_clause = parse_if_clause(inner)?;
            Ok(Command {
                inner: CommandInner::If(if_clause),
                redirect: None,
            })
        }
        Rule::while_clause => {
            Err(miette!("Unsupported compound command while_clause"))
        }
        Rule::until_clause => {
            Err(miette!("Unsupported compound command until_clause"))
        }
        Rule::ARITHMETIC_EXPRESSION => {
            let arithmetic_expression = parse_arithmetic_expression(inner)?;
            Ok(Command {
                inner: CommandInner::ArithmeticExpression(
                    arithmetic_expression,
                ),
                redirect: None,
            })
        }
        _ => Err(miette!(
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
        Err(miette!("Unexpected end of input in subshell"))
    }
}

fn parse_if_clause(pair: Pair<Rule>) -> Result<IfClause> {
    let mut inner = pair.into_inner();
    let condition = inner
        .next()
        .ok_or_else(|| miette!("Expected condition after If"))?;
    let condition = parse_conditional_expression(condition)?;

    let then_body_pair = inner
        .next()
        .ok_or_else(|| miette!("Expected then body after If"))?;
    let then_body = parse_complete_command(then_body_pair)?;

    let else_part = match inner.next() {
        Some(else_pair) => Some(parse_else_part(else_pair)?),
        None => None,
    };

    Ok(IfClause {
        condition,
        then_body,
        else_part,
    })
}

fn parse_else_part(pair: Pair<Rule>) -> Result<ElsePart> {
    let mut inner = pair.into_inner();

    let keyword = inner
        .next()
        .ok_or_else(|| miette!("Expected ELSE or ELIF keyword"))?;

    match keyword.as_rule() {
        Rule::Elif => {
            let condition = inner
                .next()
                .ok_or_else(|| miette!("Expected condition after Elif"))?;
            let condition = parse_conditional_expression(condition)?;

            let then_body_pair = inner
                .next()
                .ok_or_else(|| miette!("Expected then body after Elif"))?;
            let then_body = parse_complete_command(then_body_pair)?;

            let else_part = match inner.next() {
                Some(else_pair) => Some(parse_else_part(else_pair)?),
                None => None,
            };

            Ok(ElsePart::Elif(Box::new(IfClause {
                condition,
                then_body,
                else_part,
            })))
        }
        Rule::Else => {
            let body_pair = inner
                .next()
                .ok_or_else(|| miette!("Expected body after Else"))?;
            let body = parse_complete_command(body_pair)?;
            Ok(ElsePart::Else(body))
        }
        _ => Err(miette!(
            "Unexpected rule in else_part: {:?}",
            keyword.as_rule()
        )),
    }
}

fn parse_conditional_expression(pair: Pair<Rule>) -> Result<Condition> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| miette!("Expected conditional expression content"))?;

    match inner.as_rule() {
        Rule::unary_conditional_expression => {
            parse_unary_conditional_expression(inner)
        }
        Rule::binary_conditional_expression => {
            parse_binary_conditional_expression(inner)
        }
        _ => Err(miette!(
            "Unexpected rule in conditional expression: {:?}",
            inner.as_rule()
        )),
    }
}

fn parse_unary_conditional_expression(pair: Pair<Rule>) -> Result<Condition> {
    let mut inner = pair.into_inner();
    let operator = inner.next().ok_or_else(|| miette!("Expected operator"))?;
    let operand = inner.next().ok_or_else(|| miette!("Expected operand"))?;

    let op = match operator.as_rule() {
        Rule::string_conditional_op => match operator.as_str() {
            "-n" => UnaryOp::NonEmptyString,
            "-z" => UnaryOp::EmptyString,
            _ => {
                return Err(miette!(
                    "Unexpected string conditional operator: {}",
                    operator.as_str()
                ))
            }
        },
        Rule::file_conditional_op => match operator.as_str() {
            "-a" => UnaryOp::FileExists,
            "-b" => UnaryOp::BlockSpecial,
            "-c" => UnaryOp::CharSpecial,
            "-d" => UnaryOp::Directory,
            "-f" => UnaryOp::RegularFile,
            "-g" => UnaryOp::SetGroupId,
            "-h" => UnaryOp::SymbolicLink,
            "-k" => UnaryOp::StickyBit,
            "-p" => UnaryOp::NamedPipe,
            "-r" => UnaryOp::Readable,
            "-s" => UnaryOp::SizeNonZero,
            "-u" => UnaryOp::SetUserId,
            "-w" => UnaryOp::Writable,
            "-x" => UnaryOp::Executable,
            "-G" => UnaryOp::OwnedByEffectiveGroupId,
            "-L" => UnaryOp::SymbolicLink,
            "-N" => UnaryOp::ModifiedSinceLastRead,
            "-O" => UnaryOp::OwnedByEffectiveUserId,
            "-S" => UnaryOp::Socket,
            _ => {
                return Err(miette!(
                    "Unexpected file conditional operator: {}",
                    operator.as_str()
                ))
            }
        },
        Rule::variable_conditional_op => match operator.as_str() {
            "-v" => UnaryOp::VariableSet,
            "-R" => UnaryOp::VariableNameReference,

            _ => {
                return Err(miette!(
                    "Unexpected variable conditional operator: {}",
                    operator.as_str()
                ))
            }
        },
        _ => {
            return Err(miette!(
                "Unexpected unary conditional operator rule: {:?}",
                operator.as_rule()
            ))
        }
    };

    let right = parse_word(operand)?;

    Ok(Condition {
        condition_inner: ConditionInner::Unary {
            op: Some(op),
            right,
        },
    })
}

fn parse_binary_conditional_expression(pair: Pair<Rule>) -> Result<Condition> {
    let mut inner = pair.into_inner();
    let left = inner
        .next()
        .ok_or_else(|| miette!("Expected left operand"))?;
    let operator = inner.next().ok_or_else(|| miette!("Expected operator"))?;
    let right = inner
        .next()
        .ok_or_else(|| miette!("Expected right operand"))?;

    let left_word = parse_word(left)?;
    let right_word = parse_word(right)?;

    let op = match operator.as_rule() {
        Rule::binary_bash_conditional_op => match operator.as_str() {
            "==" => BinaryOp::Equal,
            "=" => BinaryOp::Equal,
            "!=" => BinaryOp::NotEqual,
            "<" => BinaryOp::LessThan,
            ">" => BinaryOp::GreaterThan,
            _ => {
                return Err(miette!(
                    "Unexpected string conditional operator: {}",
                    operator.as_str()
                ))
            }
        },
        Rule::binary_posix_conditional_op => match operator.as_str() {
            "-eq" => BinaryOp::Equal,
            "-ne" => BinaryOp::NotEqual,
            "-lt" => BinaryOp::LessThan,
            "-le" => BinaryOp::LessThanOrEqual,
            "-gt" => BinaryOp::GreaterThan,
            "-ge" => BinaryOp::GreaterThanOrEqual,
            _ => {
                return Err(miette!(
                    "Unexpected arithmetic conditional operator: {}",
                    operator.as_str()
                ))
            }
        },
        _ => {
            return Err(miette!(
                "Unexpected operator rule: {:?}",
                operator.as_rule()
            ))
        }
    };

    Ok(Condition {
        condition_inner: ConditionInner::Binary {
            left: left_word,
            op,
            right: right_word,
        },
    })
}

fn parse_word(pair: Pair<Rule>) -> Result<Word> {
    let mut parts = Vec::new();

    match pair.as_rule() {
        Rule::UNQUOTED_PENDING_WORD => {
            for part in pair.into_inner() {
                match part.as_rule() {
                    Rule::EXIT_STATUS => parts.push(WordPart::ExitStatus),
                    Rule::UNQUOTED_CHAR => {
                        if let Some(WordPart::Text(ref mut text)) =
                            parts.last_mut()
                        {
                            text.push(part.as_str().chars().next().unwrap());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    Rule::UNQUOTED_ESCAPE_CHAR => {
                        let mut chars = part.as_str().chars();
                        let mut escaped_char = String::new();
                        if let Some(c) = chars.next() {
                            match c {
                                '\\' => {
                                    let next_char =
                                        chars.next().unwrap_or('\0');
                                    escaped_char.push(next_char);
                                }
                                _ => {
                                    escaped_char.push(c);
                                    break;
                                }
                            }
                        }
                        if let Some(WordPart::Text(ref mut text)) =
                            parts.last_mut()
                        {
                            text.push_str(&escaped_char);
                        } else {
                            parts.push(WordPart::Text(escaped_char));
                        }
                    }
                    Rule::SUB_COMMAND => {
                        let command = parse_complete_command(
                            part.into_inner().next().unwrap(),
                        )?;
                        parts.push(WordPart::Command(command));
                    }
                    Rule::VARIABLE_EXPANSION => {
                        let variable_expansion =
                            parse_variable_expansion(part)?;
                        parts.push(variable_expansion);
                    }
                    Rule::QUOTED_WORD => {
                        let quoted = parse_quoted_word(part)?;
                        parts.push(quoted);
                    }
                    Rule::TILDE_PREFIX => {
                        let tilde_prefix = parse_tilde_prefix(part)?;
                        parts.push(tilde_prefix);
                    }
                    Rule::ARITHMETIC_EXPRESSION => {
                        let arithmetic_expression =
                            parse_arithmetic_expression(part)?;
                        parts.push(WordPart::Arithmetic(arithmetic_expression));
                    }
                    _ => {
                        return Err(miette!(
                            "Unexpected rule in UNQUOTED_PENDING_WORD: {:?}",
                            part.as_rule()
                        ));
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
                        if let Some(WordPart::Text(ref mut text)) =
                            parts.last_mut()
                        {
                            text.push(part.as_str().chars().next().unwrap());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    Rule::VARIABLE => parts.push(WordPart::Variable(
                        part.as_str().to_string(),
                        None,
                    )),
                    Rule::UNQUOTED_CHAR => {
                        if let Some(WordPart::Text(ref mut text)) =
                            parts.last_mut()
                        {
                            text.push(part.as_str().chars().next().unwrap());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    Rule::QUOTED_WORD => {
                        let quoted = parse_quoted_word(part)?;
                        parts.push(quoted);
                    }
                    Rule::TILDE_PREFIX => {
                        let tilde_prefix = parse_tilde_prefix(part)?;
                        parts.push(tilde_prefix);
                    }
                    Rule::ARITHMETIC_EXPRESSION => {
                        let arithmetic_expression =
                            parse_arithmetic_expression(part)?;
                        parts.push(WordPart::Arithmetic(arithmetic_expression));
                    }
                    _ => {
                        return Err(miette!(
                            "Unexpected rule in FILE_NAME_PENDING_WORD: {:?}",
                            part.as_rule()
                        ));
                    }
                }
            }
        }
        Rule::PARAMETER_PENDING_WORD => {
            for part in pair.into_inner() {
                match part.as_rule() {
                    Rule::PARAMETER_ESCAPE_CHAR => {
                        let mut chars = part.as_str().chars();
                        let mut escaped_char = String::new();
                        if let Some(c) = chars.next() {
                            match c {
                                '\\' => {
                                    let next_char =
                                        chars.next().unwrap_or('\0');
                                    escaped_char.push(next_char);
                                }
                                _ => {
                                    escaped_char.push(c);
                                    break;
                                }
                            }
                        }
                        if let Some(WordPart::Text(ref mut text)) =
                            parts.last_mut()
                        {
                            text.push_str(&escaped_char);
                        } else {
                            parts.push(WordPart::Text(escaped_char));
                        }
                    }
                    Rule::VARIABLE_EXPANSION => {
                        let variable_expansion =
                            parse_variable_expansion(part)?;
                        parts.push(variable_expansion);
                    }
                    Rule::QUOTED_WORD => {
                        let quoted = parse_quoted_word(part)?;
                        parts.push(quoted);
                    }
                    Rule::TILDE_PREFIX => {
                        let tilde_prefix = parse_tilde_prefix(part)?;
                        parts.push(tilde_prefix);
                    }
                    Rule::ARITHMETIC_EXPRESSION => {
                        let arithmetic_expression =
                            parse_arithmetic_expression(part)?;
                        parts.push(WordPart::Arithmetic(arithmetic_expression));
                    }
                    Rule::QUOTED_CHAR => {
                        if let Some(WordPart::Text(ref mut s)) =
                            parts.last_mut()
                        {
                            s.push_str(part.as_str());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    _ => {
                        return Err(miette!(
                            "Unexpected rule in PARAMETER_PENDING_WORD: {:?}",
                            part.as_rule()
                        ));
                    }
                }
            }
        }
        // This is the bare _name_ of a variable, not a variable expansion
        Rule::VARIABLE => parts.push(WordPart::Text(pair.as_str().to_string())),
        _ => {
            return Err(miette!(
                "Unexpected rule in word: {:?}",
                pair.as_rule()
            ));
        }
    }

    if parts.is_empty() {
        Ok(Word::new_empty())
    } else {
        Ok(Word::new(parts))
    }
}

fn parse_arithmetic_expression(pair: Pair<Rule>) -> Result<Arithmetic> {
    assert!(pair.as_rule() == Rule::ARITHMETIC_EXPRESSION);
    let inner = pair.into_inner().next().unwrap();
    let parts = parse_arithmetic_sequence(inner)?;
    Ok(Arithmetic { parts })
}

fn parse_arithmetic_sequence(pair: Pair<Rule>) -> Result<Vec<ArithmeticPart>> {
    assert!(pair.as_rule() == Rule::arithmetic_sequence);
    let mut parts = Vec::new();
    for expr in pair.into_inner() {
        parts.push(parse_arithmetic_expr(expr)?);
    }
    Ok(parts)
}

fn parse_arithmetic_expr(pair: Pair<Rule>) -> Result<ArithmeticPart> {
    ARITHMETIC_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::parentheses_expr => {
                let inner = primary.into_inner().next().unwrap();
                let parts = parse_arithmetic_sequence(inner)?;
                Ok(ArithmeticPart::ParenthesesExpr(Box::new(Arithmetic {
                    parts,
                })))
            }
            Rule::variable_assignment => {
                let mut inner = primary.into_inner();
                let name = inner.next().unwrap().as_str().to_string();
                let op = inner.next().unwrap();

                let value = parse_arithmetic_expr(inner.next().unwrap())?;
                Ok(ArithmeticPart::VariableAssignment {
                    name,
                    op: match op.as_rule() {
                        Rule::assign => AssignmentOp::Assign,
                        Rule::multiply_assign => AssignmentOp::MultiplyAssign,
                        Rule::divide_assign => AssignmentOp::DivideAssign,
                        Rule::modulo_assign => AssignmentOp::ModuloAssign,
                        Rule::add_assign => AssignmentOp::AddAssign,
                        Rule::subtract_assign => AssignmentOp::SubtractAssign,
                        Rule::left_shift_assign => {
                            AssignmentOp::LeftShiftAssign
                        }
                        Rule::right_shift_assign => {
                            AssignmentOp::RightShiftAssign
                        }
                        _ => {
                            return Err(miette!(
                                "Unexpected assignment operator: {:?}",
                                op.as_rule()
                            ));
                        }
                    },
                    value: Box::new(value),
                })
            }
            Rule::triple_conditional_expr => {
                let mut inner = primary.into_inner();
                let condition = parse_arithmetic_expr(inner.next().unwrap())?;
                let true_expr = parse_arithmetic_expr(inner.next().unwrap())?;
                let false_expr = parse_arithmetic_expr(inner.next().unwrap())?;
                Ok(ArithmeticPart::TripleConditionalExpr {
                    condition: Box::new(condition),
                    true_expr: Box::new(true_expr),
                    false_expr: Box::new(false_expr),
                })
            }
            Rule::unary_arithmetic_expr => parse_unary_arithmetic_expr(primary),
            Rule::VARIABLE => {
                Ok(ArithmeticPart::Variable(primary.as_str().to_string()))
            }
            Rule::NUMBER => {
                Ok(ArithmeticPart::Number(primary.as_str().to_string()))
            }
            _ => Err(miette!(
                "Unexpected rule in arithmetic expression: {:?}",
                primary.as_rule()
            )),
        })
        .map_infix(|lhs, op, rhs| {
            let operator = match op.as_rule() {
                Rule::add => BinaryArithmeticOp::Add,
                Rule::subtract => BinaryArithmeticOp::Subtract,
                Rule::multiply => BinaryArithmeticOp::Multiply,
                Rule::divide => BinaryArithmeticOp::Divide,
                Rule::modulo => BinaryArithmeticOp::Modulo,
                Rule::power => BinaryArithmeticOp::Power,
                Rule::left_shift => BinaryArithmeticOp::LeftShift,
                Rule::right_shift => BinaryArithmeticOp::RightShift,
                Rule::bitwise_and => BinaryArithmeticOp::BitwiseAnd,
                Rule::bitwise_xor => BinaryArithmeticOp::BitwiseXor,
                Rule::bitwise_or => BinaryArithmeticOp::BitwiseOr,
                Rule::logical_and => BinaryArithmeticOp::LogicalAnd,
                Rule::logical_or => BinaryArithmeticOp::LogicalOr,
                _ => {
                    return Err(miette!(
                        "Unexpected infix operator: {:?}",
                        op.as_rule()
                    ))
                }
            };
            Ok(ArithmeticPart::BinaryArithmeticExpr {
                left: Box::new(lhs?),
                operator,
                right: Box::new(rhs?),
            })
        })
        .parse(pair.into_inner())
}

fn parse_unary_arithmetic_expr(pair: Pair<Rule>) -> Result<ArithmeticPart> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::unary_arithmetic_op => {
            let op = parse_unary_arithmetic_op(first)?;
            let operand = parse_arithmetic_expr(inner.next().unwrap())?;
            Ok(ArithmeticPart::UnaryArithmeticExpr {
                operator: op,
                operand: Box::new(operand),
            })
        }
        Rule::post_arithmetic_op => {
            let operand = parse_arithmetic_expr(inner.next().unwrap())?;
            let op = parse_post_arithmetic_op(first)?;
            Ok(ArithmeticPart::PostArithmeticExpr {
                operand: Box::new(operand),
                operator: op,
            })
        }
        _ => {
            let operand = parse_arithmetic_expr(first)?;
            let op = parse_post_arithmetic_op(inner.next().unwrap())?;
            Ok(ArithmeticPart::PostArithmeticExpr {
                operand: Box::new(operand),
                operator: op,
            })
        }
    }
}

fn parse_unary_arithmetic_op(pair: Pair<Rule>) -> Result<UnaryArithmeticOp> {
    match pair.as_str() {
        "+" => Ok(UnaryArithmeticOp::Plus),
        "-" => Ok(UnaryArithmeticOp::Minus),
        "!" => Ok(UnaryArithmeticOp::LogicalNot),
        "~" => Ok(UnaryArithmeticOp::BitwiseNot),
        _ => Err(miette!(
            "Invalid unary arithmetic operator: {}",
            pair.as_str()
        )),
    }
}

fn parse_post_arithmetic_op(pair: Pair<Rule>) -> Result<PostArithmeticOp> {
    match pair.as_str() {
        "++" => Ok(PostArithmeticOp::Increment),
        "--" => Ok(PostArithmeticOp::Decrement),
        _ => Err(miette!(
            "Invalid post arithmetic operator: {}",
            pair.as_str()
        )),
    }
}

fn parse_variable_expansion(part: Pair<Rule>) -> Result<WordPart> {
    let mut inner = part.into_inner();
    let variable = inner
        .next()
        .ok_or_else(|| miette!("Expected variable name"))?;
    let variable_name = variable.as_str().to_string();

    let modifier = inner.next();
    let parsed_modifier = if let Some(modifier) = modifier {
        match modifier.as_rule() {
            Rule::VAR_SUBSTRING => {
                let mut numbers = modifier.into_inner();
                let begin: Word = if let Some(n) = numbers.next() {
                    parse_word(n)?
                } else {
                    return Err(miette!(
                        "Expected a number for substring begin"
                    ));
                };

                let length = if let Some(len_word) = numbers.next() {
                    Some(parse_word(len_word)?)
                } else {
                    None
                };
                Some(Box::new(VariableModifier::Substring { begin, length }))
            }
            Rule::VAR_DEFAULT_VALUE => {
                let value = if let Some(val) = modifier.into_inner().next() {
                    parse_word(val)?
                } else {
                    Word::new_empty()
                };
                Some(Box::new(VariableModifier::DefaultValue(value)))
            }
            Rule::VAR_ASSIGN_DEFAULT => {
                let value = modifier.into_inner().next().unwrap();
                Some(Box::new(VariableModifier::AssignDefault(parse_word(
                    value,
                )?)))
            }
            Rule::VAR_ALTERNATE_VALUE => {
                let value = modifier.into_inner().next().unwrap();
                Some(Box::new(VariableModifier::AlternateValue(parse_word(
                    value,
                )?)))
            }
            _ => {
                return Err(miette!(
                    "Unexpected rule in variable expansion modifier: {:?}",
                    modifier.as_rule()
                ));
            }
        }
    } else {
        None
    };
    Ok(WordPart::Variable(variable_name, parsed_modifier))
}

fn parse_tilde_prefix(pair: Pair<Rule>) -> Result<WordPart> {
    let tilde_prefix_str = pair.as_str();
    let user = if tilde_prefix_str.len() > 1 {
        Some(tilde_prefix_str[1..].to_string())
    } else {
        None
    };
    let tilde_prefix = TildePrefix::new(user);
    Ok(WordPart::Tilde(tilde_prefix))
}

fn parse_quoted_word(pair: Pair<Rule>) -> Result<WordPart> {
    let mut parts = Vec::new();
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::DOUBLE_QUOTED => {
            let inner = inner.into_inner().next().unwrap();
            for part in inner.into_inner() {
                match part.as_rule() {
                    Rule::EXIT_STATUS => {
                        parts.push(WordPart::Text("$?".to_string()))
                    }
                    Rule::QUOTED_ESCAPE_CHAR => {
                        if let Some(WordPart::Text(ref mut s)) =
                            parts.last_mut()
                        {
                            s.push_str(part.as_str());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    Rule::SUB_COMMAND => {
                        let command = parse_complete_command(
                            part.into_inner().next().unwrap(),
                        )?;
                        parts.push(WordPart::Command(command));
                    }
                    Rule::VARIABLE_EXPANSION => {
                        let variable_expansion =
                            parse_variable_expansion(part)?;
                        parts.push(variable_expansion);
                    }
                    Rule::QUOTED_CHAR => {
                        if let Some(WordPart::Text(ref mut s)) =
                            parts.last_mut()
                        {
                            s.push_str(part.as_str());
                        } else {
                            parts.push(WordPart::Text(
                                part.as_str().to_string(),
                            ));
                        }
                    }
                    _ => {
                        return Err(miette!(
                            "Unexpected rule in DOUBLE_QUOTED: {:?}",
                            part.as_rule()
                        ));
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
        _ => Err(miette!(
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
        .ok_or_else(|| miette!("Expected variable name"))?
        .as_str()
        .to_string();

    // Get the value of the environment variable
    let word_value = if let Some(value) = parts.next() {
        parse_assignment_value(value)
            .context("Failed to parse assignment value")?
    } else {
        Word::new_empty()
    };

    Ok(EnvVar {
        name,
        value: word_value,
    })
}

fn parse_assignment_value(pair: Pair<Rule>) -> Result<Word> {
    let mut parts = Vec::new();

    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::ASSIGNMENT_TILDE_PREFIX => {
                let tilde_prefix = parse_tilde_prefix(part)
                    .context("Failed to parse tilde prefix")?;
                parts.push(tilde_prefix);
            }
            Rule::UNQUOTED_PENDING_WORD => {
                let word_parts = parse_word(part)?;
                parts.extend(word_parts.into_parts());
            }
            _ => {
                return Err(miette!(
                    "Unexpected rule in assignment value: {:?}",
                    part.as_rule()
                ));
            }
        }
    }

    Ok(Word::new(parts))
}

fn parse_io_redirect(pair: Pair<Rule>) -> Result<Redirect> {
    let mut inner = pair.into_inner();

    // Parse the optional IO number or AMPERSAND
    let (maybe_fd, op_and_file) = match inner.next() {
        Some(p) if p.as_rule() == Rule::IO_NUMBER => (
            Some(RedirectFd::Fd(p.as_str().parse::<u32>().unwrap())),
            inner.next().ok_or_else(|| {
                miette!("Expected redirection operator after IO number")
            })?,
        ),
        Some(p) if p.as_rule() == Rule::AMPERSAND => (
            Some(RedirectFd::StdoutStderr),
            inner.next().ok_or_else(|| {
                miette!("Expected redirection operator after &")
            })?,
        ),
        Some(p) => (None, p),
        None => return Err(miette!("Unexpected end of input in io_redirect")),
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
        .ok_or_else(|| miette!("Expected redirection operator"))?;
    let filename = inner.next().ok_or_else(|| {
        miette!("Expected filename after redirection operator")
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
                return Err(miette!(
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
            return Err(miette!(
                "Unexpected redirection operator: {:?}",
                op.as_rule()
            ))
        }
    };

    let io_file = if filename.as_rule() == Rule::FILE_NAME_PENDING_WORD {
        IoFile::Word(parse_word(filename)?)
    } else {
        return Err(miette!(
            "Unexpected filename type: {:?}",
            filename.as_rule()
        ));
    };

    Ok((redirect_op, io_file))
}

#[cfg(test)]
mod test {
    use super::*;

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

        assert!(parse(
            "deno run --allow-read=. --allow-write=./testing main.ts"
        )
        .is_ok(),);

        assert!(parse("echo \"foo\" > out.txt").is_ok());
    }
    #[test]
    fn test_sequential_list() {
        let parse_and_create = |input: &str| -> Result<SequentialList> {
            let pairs = ShellParser::parse(Rule::complete_command, input)
                .map_err(|e| miette!(e.to_string()))?
                .next()
                .unwrap();
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
                                EnvVar::new(
                                    "Name".to_string(),
                                    Word::new_word("Value"),
                                ),
                                EnvVar::new(
                                    "OtherVar".to_string(),
                                    Word::new_word("Other"),
                                ),
                            ],
                            args: vec![
                                Word::new_word("command"),
                                Word::new_word("arg1"),
                            ],
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
                        args: vec![
                            Word::new_word("export"),
                            Word::new_word("ENV6=5"),
                        ],
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
                            inner: CommandInner::Subshell(Box::new(
                                SequentialList {
                                    items: vec![SequentialListItem {
                                        is_async: false,
                                        sequence: Sequence::BooleanList(
                                            Box::new(BooleanList {
                                                current: SimpleCommand {
                                                    env_vars: vec![],
                                                    args: vec![Word::new_word(
                                                        "cmd11",
                                                    )],
                                                }
                                                .into(),
                                                op: BooleanListOperator::Or,
                                                next: SimpleCommand {
                                                    env_vars: vec![],
                                                    args: vec![Word::new_word(
                                                        "cmd12",
                                                    )],
                                                }
                                                .into(),
                                            }),
                                        ),
                                    }],
                                },
                            )),
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
                        env_vars: vec![EnvVar::new(
                            "A".to_string(),
                            Word::new_string("b"),
                        )],
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
                        Word(vec![WordPart::Variable(
                            "MY_ENV".to_string(),
                            None,
                        )]),
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
        let parse_and_create = |input: &str| -> Result<EnvVar, miette::Error> {
            let pairs = ShellParser::parse(Rule::ASSIGNMENT_WORD, input)
                .map_err(|e| miette!(e.to_string()))?
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
