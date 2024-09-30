// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use futures::future;
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use miette::IntoDiagnostic;
use miette::Error;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::parser::AssignmentOp;
use crate::parser::BinaryOp;
use crate::parser::Condition;
use crate::parser::ConditionInner;
use crate::parser::ElsePart;
use crate::parser::IoFile;
use crate::parser::RedirectOpInput;
use crate::parser::RedirectOpOutput;
use crate::parser::UnaryOp;
use crate::parser::VariableModifier;
use crate::shell::commands::ShellCommand;
use crate::shell::commands::ShellCommandContext;
use crate::shell::types::pipe;
use crate::shell::types::ArithmeticResult;
use crate::shell::types::ArithmeticValue;
use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;
use crate::shell::types::FutureExecuteResult;
use crate::shell::types::ShellPipeReader;
use crate::shell::types::ShellPipeWriter;
use crate::shell::types::ShellState;

use crate::parser::Arithmetic;
use crate::parser::ArithmeticPart;
use crate::parser::BinaryArithmeticOp;
use crate::parser::Command;
use crate::parser::CommandInner;
use crate::parser::IfClause;
use crate::parser::PipeSequence;
use crate::parser::PipeSequenceOperator;
use crate::parser::Pipeline;
use crate::parser::PipelineInner;
use crate::parser::Redirect;
use crate::parser::RedirectFd;
use crate::parser::RedirectOp;
use crate::parser::Sequence;
use crate::parser::SequentialList;
use crate::parser::SimpleCommand;
use crate::parser::UnaryArithmeticOp;
use crate::parser::Word;
use crate::parser::WordPart;
use crate::shell::types::WordPartsResult;
use crate::shell::types::WordResult;

use super::command::execute_unresolved_command_name;
use super::command::UnresolvedCommandName;
use super::types::CANCELLATION_EXIT_CODE;

/// Executes a `SequentialList` of commands in a deno_task_shell environment.
///
/// This function accepts a list of commands, a map of environment variables, the current working directory,
/// and a map of custom shell commands. It sets up the shell state and then calls `execute_with_pipes`
/// with the standard input, output, and error streams.
///
/// # Arguments
/// * `list` - A `SequentialList` of commands to execute.
/// * `env_vars` - A map of environment variables which are set in the shell.
/// * `cwd` - The current working directory.
/// * `custom_commands` - A map of custom shell commands and there ShellCommand implementation.
///
/// # Returns
/// The exit code of the command execution.
pub async fn execute(
  list: SequentialList,
  env_vars: HashMap<String, String>,
  cwd: &Path,
  custom_commands: HashMap<String, Rc<dyn ShellCommand>>,
) -> i32 {
  let state = ShellState::new(env_vars, cwd, custom_commands);
  execute_with_pipes(
    list,
    state,
    ShellPipeReader::stdin(),
    ShellPipeWriter::stdout(),
    ShellPipeWriter::stderr(),
  )
  .await
}

/// Executes a `SequentialList` of commands with specified input and output pipes.
///
/// This function accepts a list of commands, a shell state, and pipes for standard input, output, and error.
/// This function allows the user to retrieve the data outputted by the execution and act on it using code.
/// This is made public for the use-case of running tests with shell execution in application depending on the library.
///
/// # Arguments
///
/// * `list` - A `SequentialList` of commands to execute.
/// * `state` - The current state of the shell, including environment variables and the current directory.
/// * `stdin` - A reader for the standard input stream.
/// * `stdout` - A writer for the standard output stream.
/// * `stderr` - A writer for the standard error stream.
///
/// # Returns
///
/// The exit code of the command execution.
pub async fn execute_with_pipes(
  list: SequentialList,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
) -> i32 {
  // spawn a sequential list and pipe its output to the environment
  let result = execute_sequential_list(
    list,
    state,
    stdin,
    stdout,
    stderr,
    AsyncCommandBehavior::Wait,
  )
  .await;

  match result {
    ExecuteResult::Exit(code, _) => code,
    ExecuteResult::Continue(exit_code, _, _) => exit_code,
  }
}

#[derive(Debug, PartialEq)]
pub enum AsyncCommandBehavior {
  Wait,
  Yield,
}

/// Execute a `SequentialList` of commands in a deno_task_shell environment.
pub fn execute_sequential_list(
  list: SequentialList,
  mut state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
  async_command_behavior: AsyncCommandBehavior,
) -> FutureExecuteResult {
  async move {
    let mut final_exit_code = 0;
    let mut final_changes = Vec::new();
    let mut async_handles = Vec::new();
    let mut was_exit = false;
    for item in list.items {
      if item.is_async {
        let state = state.clone();
        let stdin = stdin.clone();
        let stdout = stdout.clone();
        let stderr = stderr.clone();
        async_handles.push(tokio::task::spawn_local(async move {
          let main_token = state.token().clone();
          let result =
            execute_sequence(item.sequence, state, stdin, stdout, stderr).await;
          let (exit_code, handles) = result.into_exit_code_and_handles();
          wait_handles(exit_code, handles, main_token).await
        }));
      } else {
        let result = execute_sequence(
          item.sequence,
          state.clone(),
          stdin.clone(),
          stdout.clone(),
          stderr.clone(),
        )
        .await;
        match result {
          ExecuteResult::Exit(exit_code, handles) => {
            async_handles.extend(handles);
            final_exit_code = exit_code;
            was_exit = true;
            break;
          }
          ExecuteResult::Continue(exit_code, changes, handles) => {
            state.apply_changes(&changes);
            state.apply_env_var("?", &exit_code.to_string());
            final_changes.extend(changes);
            async_handles.extend(handles);
            // use the final sequential item's exit code
            final_exit_code = exit_code;
          }
        }
      }
    }

    // wait for async commands to complete
    if async_command_behavior == AsyncCommandBehavior::Wait {
      final_exit_code = wait_handles(
        final_exit_code,
        std::mem::take(&mut async_handles),
        state.token().clone(),
      )
      .await;
    }

    if was_exit {
      ExecuteResult::Exit(final_exit_code, async_handles)
    } else {
      ExecuteResult::Continue(final_exit_code, final_changes, async_handles)
    }
  }
  .boxed_local()
}

async fn wait_handles(
  mut exit_code: i32,
  mut handles: Vec<JoinHandle<i32>>,
  token: CancellationToken,
) -> i32 {
  if exit_code != 0 {
    token.cancel();
  }
  while !handles.is_empty() {
    let result = futures::future::select_all(handles).await;

    // prefer the first non-zero then non-cancellation exit code
    let new_exit_code = result.0.unwrap();
    if matches!(exit_code, 0 | CANCELLATION_EXIT_CODE) && new_exit_code != 0 {
      exit_code = new_exit_code;
    }

    handles = result.2;
  }
  exit_code
}

fn execute_sequence(
  sequence: Sequence,
  mut state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  mut stderr: ShellPipeWriter,
) -> FutureExecuteResult {
  // requires boxed async because of recursive async
  async move {
    match sequence {
      Sequence::ShellVar(var) => ExecuteResult::Continue(
        0,
        vec![EnvChange::SetShellVar(
          var.name,
          match evaluate_word(var.value, &state, stdin, stderr.clone()).await {
            Ok(value) => value.into(),
            Err(err) => {
              return err.into_exit_code(&mut stderr);
            }
          },
        )],
        Vec::new(),
      ),
      Sequence::BooleanList(list) => {
        let mut changes = vec![];
        let first_result = execute_sequence(
          list.current,
          state.clone(),
          stdin.clone(),
          stdout.clone(),
          stderr.clone(),
        )
        .await;
        let (exit_code, mut async_handles) = match first_result {
          ExecuteResult::Exit(_, _) => return first_result,
          ExecuteResult::Continue(exit_code, sub_changes, async_handles) => {
            state.apply_env_var("?", &exit_code.to_string());
            state.apply_changes(&sub_changes);
            changes.extend(sub_changes);
            (exit_code, async_handles)
          }
        };

        let next = if list.op.moves_next_for_exit_code(exit_code) {
          Some(list.next)
        } else {
          let mut next = list.next;
          loop {
            // boolean lists always move right on the tree
            match next {
              Sequence::BooleanList(list) => {
                if list.op.moves_next_for_exit_code(exit_code) {
                  break Some(list.next);
                }
                next = list.next;
              }
              _ => break None,
            }
          }
        };
        if let Some(next) = next {
          let next_result =
            execute_sequence(next, state, stdin, stdout, stderr).await;
          match next_result {
            ExecuteResult::Exit(code, sub_handles) => {
              async_handles.extend(sub_handles);
              ExecuteResult::Exit(code, async_handles)
            }
            ExecuteResult::Continue(exit_code, sub_changes, sub_handles) => {
              changes.extend(sub_changes);
              async_handles.extend(sub_handles);
              ExecuteResult::Continue(exit_code, changes, async_handles)
            }
          }
        } else {
          ExecuteResult::Continue(exit_code, changes, async_handles)
        }
      }
      Sequence::Pipeline(pipeline) => {
        execute_pipeline(pipeline, state, stdin, stdout, stderr).await
      }
    }
  }
  .boxed_local()
}

async fn execute_pipeline(
  pipeline: Pipeline,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
) -> ExecuteResult {
  let result =
    execute_pipeline_inner(pipeline.inner, state, stdin, stdout, stderr).await;
  if pipeline.negated {
    match result {
      ExecuteResult::Exit(code, handles) => ExecuteResult::Exit(code, handles),
      ExecuteResult::Continue(code, changes, handles) => {
        let new_code = if code == 0 { 1 } else { 0 };
        ExecuteResult::Continue(new_code, changes, handles)
      }
    }
  } else {
    result
  }
}

async fn execute_pipeline_inner(
  pipeline: PipelineInner,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
) -> ExecuteResult {
  match pipeline {
    PipelineInner::Command(command) => {
      execute_command(command, state, stdin, stdout, stderr).await
    }
    PipelineInner::PipeSequence(pipe_sequence) => {
      execute_pipe_sequence(*pipe_sequence, state, stdin, stdout, stderr).await
    }
  }
}

#[derive(Debug)]
enum RedirectPipe {
  Input(ShellPipeReader),
  Output(ShellPipeWriter),
}

async fn resolve_redirect_pipe(
  redirect: &Redirect,
  state: &ShellState,
  stdin: &ShellPipeReader,
  stdout: &ShellPipeWriter,
  stderr: &mut ShellPipeWriter,
) -> Result<RedirectPipe, ExecuteResult> {
  match redirect.io_file.clone() {
    IoFile::Word(word) => {
      resolve_redirect_word_pipe(word, &redirect.op, state, stdin, stderr).await
    }
    IoFile::Fd(fd) => match &redirect.op {
      RedirectOp::Input(RedirectOpInput::Redirect) => {
        let _ = stderr.write_line(
            "deno_task_shell: input redirecting file descriptors is not implemented",
          );
        Err(ExecuteResult::from_exit_code(1))
      }
      RedirectOp::Output(_op) => match fd {
        1 => Ok(RedirectPipe::Output(stdout.clone())),
        2 => Ok(RedirectPipe::Output(stderr.clone())),
        _ => {
          let _ = stderr.write_line(
            "deno_task_shell: output redirecting file descriptors beyond stdout and stderr is not implemented",
          );
          Err(ExecuteResult::from_exit_code(1))
        }
      },
    },
  }
}

async fn resolve_redirect_word_pipe(
  word: Word,
  redirect_op: &RedirectOp,
  state: &ShellState,
  stdin: &ShellPipeReader,
  stderr: &mut ShellPipeWriter,
) -> Result<RedirectPipe, ExecuteResult> {
  fn handle_std_result(
    output_path: &Path,
    std_file_result: std::io::Result<std::fs::File>,
    stderr: &mut ShellPipeWriter,
  ) -> Result<std::fs::File, ExecuteResult> {
    match std_file_result {
      Ok(std_file) => Ok(std_file),
      Err(err) => {
        let _ = stderr.write_line(&format!(
          "error opening file for redirect ({}). {:#}",
          output_path.display(),
          err
        ));
        Err(ExecuteResult::from_exit_code(1))
      }
    }
  }

  let words = evaluate_word_parts(
    word.into_parts(),
    state,
    stdin.clone(),
    stderr.clone(),
  )
  .await;
  let words = match words {
    Ok(word) => word,
    Err(err) => {
      return Err(err.into_exit_code(stderr));
    }
  };
  // edge case that's not supported
  if words.value.is_empty() {
    let _ = stderr.write_line("redirect path must be 1 argument, but found 0");
    return Err(ExecuteResult::from_exit_code(1));
  } else if words.value.len() > 1 {
    let _ = stderr.write_line(&format!(
      concat!(
        "redirect path must be 1 argument, but found {0} ({1}). ",
        "Did you mean to quote it (ex. \"{1}\")?"
      ),
      words.value.len(),
      words.join(" ")
    ));
    return Err(ExecuteResult::from_exit_code(1));
  }
  let output_path = &words.value[0];

  match &redirect_op {
    RedirectOp::Input(RedirectOpInput::Redirect) => {
      let output_path = state.cwd().join(output_path);
      let std_file_result =
        std::fs::OpenOptions::new().read(true).open(&output_path);
      handle_std_result(&output_path, std_file_result, stderr).map(|std_file| {
        RedirectPipe::Input(ShellPipeReader::from_std(std_file))
      })
    }
    RedirectOp::Output(op) => {
      // cross platform suppress output
      if output_path == "/dev/null" {
        return Ok(RedirectPipe::Output(ShellPipeWriter::null()));
      }
      let output_path = state.cwd().join(output_path);
      let is_append = *op == RedirectOpOutput::Append;
      let std_file_result = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(is_append)
        .truncate(!is_append)
        .open(&output_path);
      handle_std_result(&output_path, std_file_result, stderr).map(|std_file| {
        RedirectPipe::Output(ShellPipeWriter::from_std(std_file))
      })
    }
  }
}

async fn execute_command(
  command: Command,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  mut stderr: ShellPipeWriter,
) -> ExecuteResult {
  let (stdin, stdout, mut stderr) = if let Some(redirect) = &command.redirect {
    let pipe = match resolve_redirect_pipe(
      redirect,
      &state,
      &stdin,
      &stdout,
      &mut stderr,
    )
    .await
    {
      Ok(value) => value,
      Err(value) => return value,
    };
    match pipe {
      RedirectPipe::Input(pipe) => match redirect.maybe_fd {
        Some(_) => {
          let _ = stderr.write_line(
            "input redirects with file descriptors are not supported",
          );
          return ExecuteResult::from_exit_code(1);
        }
        None => (pipe, stdout, stderr),
      },
      RedirectPipe::Output(pipe) => match redirect.maybe_fd {
        Some(RedirectFd::Fd(2)) => (stdin, stdout, pipe),
        Some(RedirectFd::Fd(1)) | None => (stdin, pipe, stderr),
        Some(RedirectFd::Fd(_)) => {
          let _ = stderr.write_line(
            "only redirecting to stdout (1) and stderr (2) is supported",
          );
          return ExecuteResult::from_exit_code(1);
        }
        Some(RedirectFd::StdoutStderr) => (stdin, pipe.clone(), pipe),
      },
    }
  } else {
    (stdin, stdout, stderr)
  };
  match command.inner {
    CommandInner::Simple(command) => {
      execute_simple_command(command, state, stdin, stdout, stderr).await
    }
    CommandInner::Subshell(list) => {
      execute_subshell(list, state, stdin, stdout, stderr).await
    }
    CommandInner::If(if_clause) => {
      execute_if_clause(if_clause, state, stdin, stdout, stderr).await
    }
    CommandInner::ArithmeticExpression(arithmetic) => {
      match execute_arithmetic_expression(arithmetic, state).await {
        Ok(result) => ExecuteResult::Continue(0, result.changes, Vec::new()),
        Err(e) => {
          let _ = stderr.write_line(&e.to_string());
          ExecuteResult::Continue(2, Vec::new(), Vec::new())
        }
      }
    }
  }
}

async fn execute_arithmetic_expression(
  arithmetic: Arithmetic,
  mut state: ShellState,
) -> Result<ArithmeticResult, Error> {
  evaluate_arithmetic(&arithmetic, &mut state).await
}

async fn evaluate_arithmetic(
  arithmetic: &Arithmetic,
  state: &mut ShellState,
) -> Result<ArithmeticResult, Error> {
  let mut result = ArithmeticResult::new(ArithmeticValue::Integer(0));
  for part in &arithmetic.parts {
    result = Box::pin(evaluate_arithmetic_part(part, state)).await?;
  }
  Ok(result)
}

async fn evaluate_arithmetic_part(
  part: &ArithmeticPart,
  state: &mut ShellState,
) -> Result<ArithmeticResult, Error> {
  match part {
    ArithmeticPart::ParenthesesExpr(expr) => {
      Box::pin(evaluate_arithmetic(expr, state)).await
    }
    ArithmeticPart::VariableAssignment { name, op, value } => {
      let val = Box::pin(evaluate_arithmetic_part(value, state)).await?;
      let applied_value = match op {
        AssignmentOp::Assign => val.clone(),
        _ => {
          let var = state
            .get_var(name)
            .ok_or_else(|| miette::miette!("Undefined variable: {}", name))?;
          let parsed_var = var.parse::<ArithmeticResult>().map_err(|e| {
            miette::miette!("Failed to parse variable '{}': {}", name, e)
          })?;
          match op {
            AssignmentOp::MultiplyAssign => val.checked_mul(&parsed_var),
            AssignmentOp::DivideAssign => val.checked_div(&parsed_var),
            AssignmentOp::ModuloAssign => val.checked_rem(&parsed_var),
            AssignmentOp::AddAssign => val.checked_add(&parsed_var),
            AssignmentOp::SubtractAssign => val.checked_sub(&parsed_var),
            AssignmentOp::LeftShiftAssign => val.checked_shl(&parsed_var),
            AssignmentOp::RightShiftAssign => val.checked_shr(&parsed_var),
            AssignmentOp::BitwiseAndAssign => val.checked_and(&parsed_var),
            AssignmentOp::BitwiseXorAssign => val.checked_xor(&parsed_var),
            AssignmentOp::BitwiseOrAssign => val.checked_or(&parsed_var),
            _ => unreachable!(),
          }?
        }
      };
      state.apply_env_var(name, &applied_value.to_string());
      Ok(
        applied_value
          .clone()
          .with_changes(vec![EnvChange::SetShellVar(
            name.clone(),
            applied_value.to_string(),
          )]),
      )
    }
    ArithmeticPart::TripleConditionalExpr {
      condition,
      true_expr,
      false_expr,
    } => {
      let cond = Box::pin(evaluate_arithmetic_part(condition, state)).await?;
      if cond.is_zero() {
        Box::pin(evaluate_arithmetic_part(true_expr, state)).await
      } else {
        Box::pin(evaluate_arithmetic_part(false_expr, state)).await
      }
    }
    ArithmeticPart::BinaryArithmeticExpr {
      left,
      operator,
      right,
    } => {
      let lhs = Box::pin(evaluate_arithmetic_part(left, state)).await?;
      let rhs = Box::pin(evaluate_arithmetic_part(right, state)).await?;
      apply_binary_op(lhs, *operator, rhs)
    }
    ArithmeticPart::BinaryConditionalExpr {
      left,
      operator,
      right,
    } => {
      let lhs = Box::pin(evaluate_arithmetic_part(left, state)).await?;
      let rhs = Box::pin(evaluate_arithmetic_part(right, state)).await?;
      apply_conditional_binary_op(lhs, operator, rhs)
    }
    ArithmeticPart::UnaryArithmeticExpr { operator, operand } => {
      let val = Box::pin(evaluate_arithmetic_part(operand, state)).await?;
      apply_unary_op(*operator, val)
    }
    ArithmeticPart::PostArithmeticExpr { operand, .. } => {
      let val = Box::pin(evaluate_arithmetic_part(operand, state)).await?;
      Ok(val)
    }
    ArithmeticPart::Variable(name) => state
      .get_var(name)
      .and_then(|s| s.parse::<ArithmeticResult>().ok())
      .ok_or_else(|| {
        miette::miette!("Undefined or non-integer variable: {}", name)
      }),
    ArithmeticPart::Number(num_str) => num_str
      .parse::<ArithmeticResult>()
      .map_err(|e| miette::miette!(e.to_string())),
  }
}

fn apply_binary_op(
  lhs: ArithmeticResult,
  op: BinaryArithmeticOp,
  rhs: ArithmeticResult,
) -> Result<ArithmeticResult, Error> {
  match op {
    BinaryArithmeticOp::Add => lhs.checked_add(&rhs),
    BinaryArithmeticOp::Subtract => lhs.checked_sub(&rhs),
    BinaryArithmeticOp::Multiply => lhs.checked_mul(&rhs),
    BinaryArithmeticOp::Divide => lhs.checked_div(&rhs),
    BinaryArithmeticOp::Modulo => lhs.checked_rem(&rhs),
    BinaryArithmeticOp::Power => lhs.checked_pow(&rhs),
    BinaryArithmeticOp::LeftShift => lhs.checked_shl(&rhs),
    BinaryArithmeticOp::RightShift => lhs.checked_shr(&rhs),
    BinaryArithmeticOp::BitwiseAnd => lhs.checked_and(&rhs),
    BinaryArithmeticOp::BitwiseXor => lhs.checked_xor(&rhs),
    BinaryArithmeticOp::BitwiseOr => lhs.checked_or(&rhs),
    BinaryArithmeticOp::LogicalAnd => Ok(if lhs.is_zero() && rhs.is_zero() {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    }),
    BinaryArithmeticOp::LogicalOr => Ok(if !lhs.is_zero() || !rhs.is_zero() {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
  }
}

fn apply_conditional_binary_op(
  lhs: ArithmeticResult,
  op: &BinaryOp,
  rhs: ArithmeticResult,
) -> Result<ArithmeticResult, Error> {
  match op {
    BinaryOp::Equal => Ok(if lhs == rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    BinaryOp::NotEqual => Ok(if lhs != rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    BinaryOp::LessThan => Ok(if lhs < rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    BinaryOp::LessThanOrEqual => Ok(if lhs <= rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    BinaryOp::GreaterThan => Ok(if lhs > rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    BinaryOp::GreaterThanOrEqual => Ok(if lhs >= rhs {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
  }
}

fn apply_unary_op(
  op: UnaryArithmeticOp,
  val: ArithmeticResult,
) -> Result<ArithmeticResult, Error> {
  match op {
    UnaryArithmeticOp::Plus => Ok(val),
    UnaryArithmeticOp::Minus => val.checked_neg(),
    UnaryArithmeticOp::LogicalNot => Ok(if val.is_zero() {
      ArithmeticResult::new(ArithmeticValue::Integer(1))
    } else {
      ArithmeticResult::new(ArithmeticValue::Integer(0))
    }),
    UnaryArithmeticOp::BitwiseNot => val.checked_not(),
  }
}

async fn execute_pipe_sequence(
  pipe_sequence: PipeSequence,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
) -> ExecuteResult {
  let mut wait_tasks = vec![];
  let mut last_output = Some(stdin);
  let mut next_inner: Option<PipelineInner> = Some(pipe_sequence.into());
  while let Some(sequence) = next_inner.take() {
    let (output_reader, output_writer) = pipe();
    let (stderr, command) = match sequence {
      PipelineInner::PipeSequence(pipe_sequence) => {
        next_inner = Some(pipe_sequence.next);
        (
          match pipe_sequence.op {
            PipeSequenceOperator::Stdout => stderr.clone(),
            PipeSequenceOperator::StdoutStderr => output_writer.clone(),
          },
          pipe_sequence.current,
        )
      }
      PipelineInner::Command(command) => (stderr.clone(), command),
    };
    wait_tasks.push(execute_command(
      command,
      state.clone(),
      last_output.take().unwrap(),
      output_writer.clone(),
      stderr.clone(),
    ));
    last_output = Some(output_reader);
  }
  let output_handle = tokio::task::spawn_blocking(|| {
    last_output.unwrap().pipe_to_sender(stdout).unwrap();
  });
  let mut results = futures::future::join_all(wait_tasks).await;
  output_handle.await.unwrap();
  let last_result = results.pop().unwrap();
  let all_handles = results.into_iter().flat_map(|r| r.into_handles());
  match last_result {
    ExecuteResult::Exit(code, mut handles) => {
      handles.extend(all_handles);
      ExecuteResult::Continue(code, Vec::new(), handles)
    }
    ExecuteResult::Continue(code, _, mut handles) => {
      handles.extend(all_handles);
      ExecuteResult::Continue(code, Vec::new(), handles)
    }
  }
}

async fn execute_subshell(
  list: Box<SequentialList>,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  stderr: ShellPipeWriter,
) -> ExecuteResult {
  let result = execute_sequential_list(
    *list,
    state,
    stdin,
    stdout,
    stderr,
    // yield async commands to the parent
    AsyncCommandBehavior::Yield,
  )
  .await;

  match result {
    ExecuteResult::Exit(code, handles) => {
      // sub shells do not cause an exit
      ExecuteResult::Continue(code, Vec::new(), handles)
    }
    ExecuteResult::Continue(code, _env_changes, handles) => {
      // env changes are not propagated
      ExecuteResult::Continue(code, Vec::new(), handles)
    }
  }
}

async fn execute_if_clause(
  if_clause: IfClause,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  mut stderr: ShellPipeWriter,
) -> ExecuteResult {
  let mut current_condition = if_clause.condition;
  let mut current_body = if_clause.then_body;
  let mut current_else = if_clause.else_part;

  loop {
    let condition_result = evaluate_condition(
      current_condition,
      &state,
      stdin.clone(),
      stderr.clone(),
    )
    .await;
    match condition_result {
      Ok(true) => {
        return execute_sequential_list(
          current_body,
          state,
          stdin,
          stdout,
          stderr,
          AsyncCommandBehavior::Yield,
        )
        .await;
      }
      Ok(false) => match current_else {
        Some(ElsePart::Elif(elif_clause)) => {
          current_condition = elif_clause.condition;
          current_body = elif_clause.then_body;
          current_else = elif_clause.else_part;
        }
        Some(ElsePart::Else(else_body)) => {
          return execute_sequential_list(
            else_body,
            state,
            stdin,
            stdout,
            stderr,
            AsyncCommandBehavior::Yield,
          )
          .await;
        }
        None => {
          return ExecuteResult::Continue(0, Vec::new(), Vec::new());
        }
      },
      Err(err) => {
        return err.into_exit_code(&mut stderr);
      }
    }
  }
}

async fn evaluate_condition(
  condition: Condition,
  state: &ShellState,
  stdin: ShellPipeReader,
  stderr: ShellPipeWriter,
) -> Result<bool, EvaluateWordTextError> {
  match condition.condition_inner {
    ConditionInner::Binary { left, op, right } => {
      let left =
        evaluate_word(left, state, stdin.clone(), stderr.clone()).await?;
      let right =
        evaluate_word(right, state, stdin.clone(), stderr.clone()).await?;

      // transform the string comparison to a numeric comparison if possible
      if let Ok(left) = Into::<String>::into(left.clone()).parse::<i64>() {
        if let Ok(right) = Into::<String>::into(right.clone()).parse::<i64>() {
          return Ok(match op {
            BinaryOp::Equal => left == right,
            BinaryOp::NotEqual => left != right,
            BinaryOp::LessThan => left < right,
            BinaryOp::LessThanOrEqual => left <= right,
            BinaryOp::GreaterThan => left > right,
            BinaryOp::GreaterThanOrEqual => left >= right,
          });
        }
      }

      match op {
        BinaryOp::Equal => Ok(left == right),
        BinaryOp::NotEqual => Ok(left != right),
        BinaryOp::LessThan => Ok(left < right),
        BinaryOp::LessThanOrEqual => Ok(left <= right),
        BinaryOp::GreaterThan => Ok(left > right),
        BinaryOp::GreaterThanOrEqual => Ok(left >= right),
      }
    }
    ConditionInner::Unary { op, right } => {
      let _right =
        evaluate_word(right, state, stdin.clone(), stderr.clone()).await?;
      match op {
        Some(UnaryOp::FileExists) => todo!(),
        Some(UnaryOp::BlockSpecial) => todo!(),
        Some(UnaryOp::CharSpecial) => todo!(),
        Some(UnaryOp::Directory) => todo!(),
        Some(UnaryOp::RegularFile) => todo!(),
        Some(UnaryOp::SetGroupId) => todo!(),
        Some(UnaryOp::SymbolicLink) => todo!(),
        Some(UnaryOp::StickyBit) => todo!(),
        Some(UnaryOp::NamedPipe) => todo!(),
        Some(UnaryOp::Readable) => todo!(),
        Some(UnaryOp::SizeNonZero) => todo!(),
        Some(UnaryOp::TerminalFd) => todo!(),
        Some(UnaryOp::SetUserId) => todo!(),
        Some(UnaryOp::Writable) => todo!(),
        Some(UnaryOp::Executable) => todo!(),
        Some(UnaryOp::OwnedByEffectiveGroupId) => todo!(),
        Some(UnaryOp::ModifiedSinceLastRead) => todo!(),
        Some(UnaryOp::OwnedByEffectiveUserId) => todo!(),
        Some(UnaryOp::Socket) => todo!(),
        Some(UnaryOp::NonEmptyString) => todo!(),
        Some(UnaryOp::EmptyString) => todo!(),
        Some(UnaryOp::VariableSet) => todo!(),
        Some(UnaryOp::VariableNameReference) => todo!(),
        None => todo!(),
      }
    }
  }
}

async fn execute_simple_command(
  command: SimpleCommand,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  mut stderr: ShellPipeWriter,
) -> ExecuteResult {
  let args =
    evaluate_args(command.args, &state, stdin.clone(), stderr.clone()).await;
  let (args, changes) = match args {
    Ok(args) => (args.value, args.changes),
    Err(err) => {
      return err.into_exit_code(&mut stderr);
    }
  };
  let mut state = state.clone();
  for env_var in command.env_vars {
    let value =
      evaluate_word(env_var.value, &state, stdin.clone(), stderr.clone()).await;
    let value = match value {
      Ok(value) => value,
      Err(err) => {
        return err.into_exit_code(&mut stderr);
      }
    };
    state.apply_env_var(&env_var.name, value.value());
  }
  let result = execute_command_args(args, state, stdin, stdout, stderr).await;
  match result {
    ExecuteResult::Exit(code, handles) => ExecuteResult::Exit(code, handles),
    ExecuteResult::Continue(code, env_changes, handles) => {
      let mut combined_changes = env_changes.clone();
      combined_changes.extend(changes);
      ExecuteResult::Continue(code, combined_changes, handles)
    }
  }
}

fn execute_command_args(
  mut args: Vec<String>,
  state: ShellState,
  stdin: ShellPipeReader,
  stdout: ShellPipeWriter,
  mut stderr: ShellPipeWriter,
) -> FutureExecuteResult {
  let command_name = if args.is_empty() {
    String::new()
  } else {
    // check if the command name is in the alias hashmap
    if let Some(value) = state.alias_map().get(&args[0]) {
      args.remove(0);
      args = value
        .iter()
        .chain(args.iter())
        .cloned()
        .collect::<Vec<String>>();
    }

    args.remove(0)
  };

  if state.token().is_cancelled() {
    Box::pin(future::ready(ExecuteResult::for_cancellation()))
  } else if let Some(stripped_name) = command_name.strip_prefix('!') {
    let _ = stderr.write_line(
        &format!(concat!(
          "History expansion is not supported:\n",
          "  {}\n",
          "  ~\n\n",
          "Perhaps you meant to add a space after the exclamation point to negate the command?\n",
          "  ! {}",
        ), command_name, stripped_name)
      );
    Box::pin(future::ready(ExecuteResult::from_exit_code(1)))
  } else {
    let command_context = ShellCommandContext {
      args,
      state,
      stdin,
      stdout,
      stderr,
      execute_command_args: Box::new(move |context| {
        execute_command_args(
          context.args,
          context.state,
          context.stdin,
          context.stdout,
          context.stderr,
        )
      }),
    };
    match command_context.state.resolve_custom_command(&command_name) {
      Some(command) => command.execute(command_context),
      None => execute_unresolved_command_name(
        UnresolvedCommandName {
          name: command_name,
          base_dir: command_context.state.cwd().to_path_buf(),
        },
        command_context,
      ),
    }
  }
}

pub async fn evaluate_args(
  args: Vec<Word>,
  state: &ShellState,
  stdin: ShellPipeReader,
  stderr: ShellPipeWriter,
) -> Result<WordPartsResult, EvaluateWordTextError> {
  let mut result = WordPartsResult::new(Vec::new(), Vec::new());
  for arg in args {
    let parts = evaluate_word_parts(
      arg.into_parts(),
      state,
      stdin.clone(),
      stderr.clone(),
    )
    .await?;
    result.extend(parts);
  }
  Ok(result)
}

async fn evaluate_word(
  word: Word,
  state: &ShellState,
  stdin: ShellPipeReader,
  stderr: ShellPipeWriter,
) -> Result<WordResult, EvaluateWordTextError> {
    Ok(evaluate_word_parts(word.into_parts(), state, stdin, stderr)
      .await?
      .into()
    )
}

#[derive(Debug, Error)]
pub enum EvaluateWordTextError {
  #[error("glob: no matches found '{}'. {}", pattern, err)]
  InvalidPattern {
    pattern: String,
    err: glob::PatternError,
  },
  #[error("glob: no matches found '{}'", pattern)]
  NoFilesMatched { pattern: String },
  #[error("Failed to get home directory")]
  FailedToGetHomeDirectory(miette::Error),
}

impl EvaluateWordTextError {
  pub fn into_exit_code(self, stderr: &mut ShellPipeWriter) -> ExecuteResult {
    let _ = stderr.write_line(&self.to_string());
    ExecuteResult::from_exit_code(1)
  }
}

impl From<miette::Error> for EvaluateWordTextError {
  fn from(err: miette::Error) -> Self {
    Self::FailedToGetHomeDirectory(err)
  }
}

impl VariableModifier {
  pub async fn apply(&self, variable: Option<&String>,
    state: &ShellState,
    stdin: ShellPipeReader,
    stderr: ShellPipeWriter,
) -> Result<Option<String>, miette::Report> {
    match self {
      VariableModifier::DefaultValue(default_value) => match variable {
        Some(v) => Ok(Some(v.to_string())),
        None => Ok(Some(evaluate_word(default_value.clone(), state, stdin, stderr).await.into_diagnostic()?.into())),
      },
      // VariableModifier::Substring { begin, length } => {
      //   if variable.is_none() {
      //     return Err(miette::miette!("Variable not found"));
      //   }
      //   let variable = variable.unwrap();
      //   let chars: Vec<char> = variable.chars().collect();
      //   let start = usize::try_from(*begin).unwrap();
      //   let end = match length {
      //     Some(len) => {
      //       (start + usize::try_from(*len).unwrap()).min(chars.len())
      //     }
      //     None => chars.len(),
      //   };
      //   Ok(Some(chars[start..end].iter().collect()))
      // },
      _ => Err(miette::miette!("Unsupported variable modifier")),
    }
  }
}

fn evaluate_word_parts(
  parts: Vec<WordPart>,
  state: &ShellState,
  stdin: ShellPipeReader,
  stderr: ShellPipeWriter,
) -> LocalBoxFuture<Result<WordPartsResult, EvaluateWordTextError>> {
  #[derive(Debug)]
  enum TextPart {
    Quoted(String),
    Text(String),
  }

  impl TextPart {
    pub fn as_str(&self) -> &str {
      match self {
        TextPart::Quoted(text) => text,
        TextPart::Text(text) => text,
      }
    }
  }

  fn text_parts_to_string(parts: Vec<TextPart>) -> String {
    let mut result =
      String::with_capacity(parts.iter().map(|p| p.as_str().len()).sum());
    for part in parts {
      result.push_str(part.as_str());
    }
    result
  }

  fn evaluate_word_text(
    state: &ShellState,
    text_parts: Vec<TextPart>,
    is_quoted: bool,
  ) -> Result<WordPartsResult, EvaluateWordTextError> {
    if !is_quoted
      && text_parts
        .iter()
        .filter_map(|p| match p {
          TextPart::Quoted(_) => None,
          TextPart::Text(text) => Some(text.as_str()),
        })
        .any(|text| text.chars().any(|c| matches!(c, '?' | '*' | '[')))
    {
      let mut current_text = String::new();
      for text_part in text_parts {
        match text_part {
          TextPart::Quoted(text) => {
            for c in text.chars() {
              match c {
                '?' | '*' | '[' | ']' => {
                  // escape because it was quoted
                  current_text.push('[');
                  current_text.push(c);
                  current_text.push(']');
                }
                _ => current_text.push(c),
              }
            }
          }
          TextPart::Text(text) => {
            current_text.push_str(&text);
          }
        }
      }
      let is_absolute = std::path::PathBuf::from(&current_text).is_absolute();
      let cwd = state.cwd();
      let pattern = if is_absolute {
        current_text
      } else {
        format!("{}/{}", cwd.display(), current_text)
      };
      let result = glob::glob_with(
        &pattern,
        glob::MatchOptions {
          // false because it should work the same way on case insensitive file systems
          case_sensitive: false,
          // true because it copies what sh does
          require_literal_separator: true,
          // true because it copies with sh does—these files are considered "hidden"
          require_literal_leading_dot: true,
        },
      );
      match result {
        Ok(paths) => {
          let paths =
            paths.into_iter().filter_map(|p| p.ok()).collect::<Vec<_>>();
          if paths.is_empty() {
            Err(EvaluateWordTextError::NoFilesMatched { pattern })
          } else {
            let paths = if is_absolute {
              paths
                .into_iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
            } else {
              paths
                .into_iter()
                .map(|p| {
                  let path = p.strip_prefix(cwd).unwrap();
                  path.display().to_string()
                })
                .collect::<Vec<_>>()
            };
            Ok(WordPartsResult::new(paths, Vec::new()))
          }
        }
        Err(err) => Err(EvaluateWordTextError::InvalidPattern { pattern, err }),
      }
    } else {
      Ok(WordPartsResult {
        value: vec![text_parts_to_string(text_parts)],
        changes: Vec::new(),
      })
    }
  }

  fn evaluate_word_parts_inner(
    parts: Vec<WordPart>,
    is_quoted: bool,
    state: &ShellState,
    stdin: ShellPipeReader,
    stderr: ShellPipeWriter,
  ) -> LocalBoxFuture<Result<WordPartsResult, EvaluateWordTextError>> {
    // recursive async, so requires boxing
    let mut changes: Vec<EnvChange> = Vec::new();

    async move {
      let mut result = WordPartsResult::new(Vec::new(), Vec::new());
      let mut current_text = Vec::new();
      for part in parts {
        let evaluation_result_text = match part {
          WordPart::Text(text) => {
            current_text.push(TextPart::Text(text));
            None
          }
          WordPart::Variable(name, modifier) => {
            let value = state.get_var(&name).map(|v| v.to_string());
            if let Some(modifier) = modifier {
              modifier.apply(value.as_ref(), state, stdin.clone(), stderr.clone()).await?
            } else {
              value
            }
          }
          WordPart::Command(list) => Some(
            evaluate_command_substitution(
              list,
              // contain cancellation to the command substitution
              &state.with_child_token(),
              stdin.clone(),
              stderr.clone(),
            )
            .await,
          ),
          WordPart::Quoted(parts) => {
            let res = evaluate_word_parts_inner(
              parts,
              true,
              state,
              stdin.clone(),
              stderr.clone(),
            )
            .await?;
            
            changes.extend(res.changes);
          
            current_text.push(TextPart::Quoted(res.into()));
            continue;
          }
          WordPart::Tilde(tilde_prefix) => {
            if tilde_prefix.only_tilde() {
              let home_str = dirs::home_dir()
                .ok_or_else(|| miette::miette!("Failed to get home directory"))?
                .display()
                .to_string();
              current_text.push(TextPart::Text(home_str));
            } else {
              todo!("tilde expansion with user name is not supported");
            }
            continue;
          }
          WordPart::Arithmetic(arithmetic) => {
            let arithmetic_result =
              execute_arithmetic_expression(arithmetic, state.clone()).await?;
            current_text.push(TextPart::Text(arithmetic_result.to_string()));
            changes.extend(arithmetic_result.changes);
            continue;
          }
          WordPart::ExitStatus => {
            let exit_code = state.last_command_exit_code();
            current_text.push(TextPart::Text(exit_code.to_string()));
            continue;
          }
        };

        // This text needs to be turned into a vector of strings.
        // For now we do a very basic string split on whitespace, but in the future
        // we should continue to improve this functionality.
        if let Some(text) = evaluation_result_text {
          let mut parts = text
            .split(' ')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| TextPart::Text(p.to_string()))
            .collect::<Vec<_>>();

          if !parts.is_empty() {
            // append the first part to the current text
            let first_part = parts.remove(0);
            current_text.push(first_part);

            if !parts.is_empty() {
              // evaluate and store the current text
              result.extend(evaluate_word_text(
                state,
                current_text,
                is_quoted,
              )?);

              // store all the parts except the last one
              for part in parts.drain(..parts.len() - 1) {
                result.extend(evaluate_word_text(
                  state,
                  vec![part],
                  is_quoted,
                )?);
              }

              // use the last part as the current text so it maybe
              // gets appended to in the future
              current_text = parts;
            }
          }
        }
      }
      if !current_text.is_empty() {
        result.extend(evaluate_word_text(state, current_text, is_quoted)?);
      }
      Ok(result)
    }
    .boxed_local()
  }

  evaluate_word_parts_inner(parts, false, state, stdin, stderr)
}

async fn evaluate_command_substitution(
  list: SequentialList,
  state: &ShellState,
  stdin: ShellPipeReader,
  stderr: ShellPipeWriter,
) -> String {
  let text = execute_with_stdout_as_text(|shell_stdout_writer| {
    execute_sequential_list(
      list,
      state.clone(),
      stdin,
      shell_stdout_writer,
      stderr,
      AsyncCommandBehavior::Wait,
    )
  })
  .await;

  // Remove the trailing newline and then replace inner newlines with a space
  // This seems to be what sh does, but I'm not entirely sure:
  //
  // > echo $(echo 1 && echo -e "\n2\n")
  // 1 2
  text
    .strip_suffix("\r\n")
    .or_else(|| text.strip_suffix('\n'))
    .unwrap_or(&text)
    .replace("\r\n", " ")
    .replace('\n', " ")
}

async fn execute_with_stdout_as_text(
  execute: impl FnOnce(ShellPipeWriter) -> FutureExecuteResult,
) -> String {
  let (shell_stdout_reader, shell_stdout_writer) = pipe();
  let spawned_output = execute(shell_stdout_writer);
  let output_handle = tokio::task::spawn_blocking(move || {
    let mut final_data = Vec::new();
    shell_stdout_reader.pipe_to(&mut final_data).unwrap();
    final_data
  });
  let _ = spawned_output.await;
  let data = output_handle.await.unwrap();
  String::from_utf8_lossy(&data).to_string()
}
