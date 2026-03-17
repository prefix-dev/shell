// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::ExecuteResult;

/// The `return` builtin exits from a function or sourced script with
/// the specified exit code (default 0). It uses a sentinel exit code
/// to propagate through the execution stack.
///
/// Currently, since functions aren't fully implemented, `return` acts
/// like `exit` for sourced scripts.
pub struct ReturnCommand;

impl ShellCommand for ReturnCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let exit_code = if context.args.is_empty() {
            // Default: return with the exit status of the last command
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

        // Use Exit variant to stop execution of the current scope
        // (function or sourced script)
        Box::pin(futures::future::ready(ExecuteResult::Exit(
            exit_code,
            Vec::new(),
            Vec::new(),
        )))
    }
}
