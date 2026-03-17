// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;
use crate::shell::types::RETURN_EXIT_CODE;

/// The `return` builtin exits from a function or sourced script with
/// the specified exit code. Uses RETURN_EXIT_CODE sentinel so the
/// function executor can catch it and extract the actual return value.
pub struct ReturnCommand;

impl ShellCommand for ReturnCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let exit_code = if context.args.is_empty() {
            context
                .state
                .get_var("?")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0)
        } else {
            match context.args[0].parse::<i32>() {
                Ok(code) => code,
                Err(_) => {
                    let _ = context.stderr.clone().write_line(&format!(
                        "return: {}: numeric argument required",
                        context.args[0]
                    ));
                    2
                }
            }
        };

        // Use RETURN_EXIT_CODE sentinel so function executor catches it.
        // Store the actual return value in $? via an env change.
        Box::pin(futures::future::ready(ExecuteResult::Exit(
            RETURN_EXIT_CODE,
            vec![EnvChange::SetShellVar(
                "?".to_string(),
                exit_code.to_string(),
            )],
            Vec::new(),
        )))
    }
}
