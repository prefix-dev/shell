// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;
use futures::FutureExt;

use crate::parser;
use crate::shell::execute::execute_sequential_list;
use crate::shell::execute::AsyncCommandBehavior;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::ExecuteResult;

pub struct EvalCommand;

impl ShellCommand for EvalCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let input = context.args.join(" ");
        if input.is_empty() {
            return Box::pin(futures::future::ready(
                ExecuteResult::from_exit_code(0),
            ));
        }

        async move {
            let parsed = match parser::parse(&input) {
                Ok(list) => list,
                Err(err) => {
                    let _ = context.stderr.clone().write_line(&format!(
                        "eval: {err}"
                    ));
                    return ExecuteResult::Continue(2, Vec::new(), Vec::new());
                }
            };

            execute_sequential_list(
                parsed,
                context.state,
                context.stdin,
                context.stdout,
                context.stderr,
                AsyncCommandBehavior::Wait,
            )
            .await
        }
        .boxed_local()
    }
}
