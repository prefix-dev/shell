use std::{ffi::OsString, fs};

use deno_task_shell::{EnvChange, ExecuteResult, ShellCommand, ShellCommandContext};
use futures::{future::LocalBoxFuture, FutureExt};

use uu_ls::uumain as uu_ls;

use crate::execute;
pub struct LsCommand;

pub struct AliasCommand;

pub struct UnAliasCommand;

pub struct SourceCommand;

impl ShellCommand for AliasCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        if context.args.len() != 1 {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)));
        }

        // parse the args
        let env_change = if let Some((alias, cmd)) = context.args[0].split_once('=') {
            vec![EnvChange::AliasCommand(alias.into(), cmd.into())]
        } else {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)));
        };

        let result = ExecuteResult::Continue(0, env_change, Vec::default());
        Box::pin(futures::future::ready(result))
    }
}

impl ShellCommand for UnAliasCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        if context.args.len() != 1 {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)));
        }

        let result = ExecuteResult::Continue(
            0,
            vec![EnvChange::UnAliasCommand(context.args[0].clone())],
            Vec::default(),
        );
        Box::pin(futures::future::ready(result))
    }
}

impl ShellCommand for LsCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        let result = execute_ls(context);
        Box::pin(futures::future::ready(result))
    }
}

fn execute_ls(context: ShellCommandContext) -> ExecuteResult {
    let mut args: Vec<OsString> = vec![OsString::from("ls"), OsString::from("--color=auto")];

    context
        .args
        .iter()
        .for_each(|arg| args.push(OsString::from(arg)));

    let exit_code = uu_ls(args.into_iter());
    ExecuteResult::from_exit_code(exit_code)
}

impl ShellCommand for SourceCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        if context.args.len() != 1 {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)));
        }

        let script = context.args[0].clone();
        // read the script
        let script_file = context.state.cwd().join(script);
        if script_file.exists() {
            // TODO turn into execute result
            let content = fs::read_to_string(script_file).unwrap();
            let mut state = context.state.clone();
            async move {
                execute::execute(&content, &mut state).await.unwrap();
                ExecuteResult::from_exit_code(0)
            }.boxed_local()
        }

        Box::pin(futures::future::ready(ExecuteResult::from_exit_code(0)))
    }
}
