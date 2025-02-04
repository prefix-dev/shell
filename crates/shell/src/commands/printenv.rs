use std::ffi::OsString;

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;
use uu_printenv::uumain as uu_printenv;

pub struct PrintEnvCommand;

impl ShellCommand for PrintEnvCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(
            match execute_printenv(&mut context) {
                Ok(_) => ExecuteResult::from_exit_code(0),
                Err(exit_code) => ExecuteResult::from_exit_code(exit_code),
            },
        ))
    }
}

fn execute_printenv(context: &mut ShellCommandContext) -> Result<(), i32> {
    let mut args: Vec<OsString> = vec![OsString::from("printenv")];

    context
        .args
        .iter()
        .for_each(|arg| args.push(OsString::from(arg)));

    let exit_code = uu_printenv(args.into_iter());
    if exit_code != 0 {
        return Err(exit_code);
    }

    Ok(())
}
