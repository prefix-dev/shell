use std::ffi::OsString;

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

use uu_ls::uumain as uu_ls;
pub struct LsCommand;

impl ShellCommand for LsCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        let result = execute_ls(context);
        Box::pin(futures::future::ready(result))
    }
}

fn execute_ls(context: ShellCommandContext) -> ExecuteResult {
    let mut args: Vec<OsString> = vec![OsString::from("ls"), OsString::from("--color=auto")];

    context.args.iter().for_each(|arg| {
        let expanded_arg = if arg.contains('~') {
            if let Some(home_dir) = dirs::home_dir() {
                arg.replace('~', home_dir.to_str().unwrap_or("~"))
            } else {
                arg.to_string()
            }
        } else {
            arg.to_string()
        };
        args.push(OsString::from(expanded_arg));
    });

    let exit_code = uu_ls(args.into_iter());
    ExecuteResult::from_exit_code(exit_code)
}
