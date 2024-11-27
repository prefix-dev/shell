// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;
use miette::bail;
use miette::Result;

use deno_task_shell::{ExecuteResult, EnvChange, ShellCommandContext, ShellCommand, ArgKind, parse_arg_kinds, ShellOptions};

pub struct SetCommand;

impl ShellCommand for SetCommand {
  fn execute(
    &self,
    mut context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    let result = match execute_set(context.args) {
      Ok((code, env_changes)) => ExecuteResult::Continue(code, env_changes, Vec::new()),
      Err(err) => {
        context.stderr.write_line(&format!("set: {err}")).unwrap();
        ExecuteResult::Exit(2, Vec::new())
      }
    };
    Box::pin(futures::future::ready(result))
  }
}

fn execute_set(args: Vec<String>) -> Result<(i32, Vec<EnvChange>)> {
  let args = parse_arg_kinds(&args);
  let mut env_changes = Vec::new();
  for arg in args {
    match arg {
      ArgKind::MinusShortFlag('e') => {
        env_changes.push(EnvChange::SetShellOptions(ShellOptions::ExitOnError, true));
      }
      ArgKind::PlusShortFlag('e') => {
        env_changes.push(EnvChange::SetShellOptions(ShellOptions::ExitOnError, false));
      }
      _ => bail!(format!("Unsupported argument: {:?}", arg)),
    }
  }
  Ok((0, env_changes))
}
