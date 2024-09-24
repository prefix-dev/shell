use std::ffi::OsString;

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;
use uu_date::uumain as uu_date;

pub struct DateCommand;

impl ShellCommand for DateCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(match execute_date(&mut context) {
            Ok(_) => ExecuteResult::from_exit_code(0),
            Err(exit_code) => ExecuteResult::from_exit_code(exit_code),
        }))
    }
}

fn execute_date(context: &mut ShellCommandContext) -> Result<(), i32> {
    let mut args: Vec<OsString> = vec![OsString::from("date")];

    context
        .args
        .iter()
        .for_each(|arg| args.push(OsString::from(arg)));

    let exit_code = uu_date(args.into_iter());
    if exit_code != 0 {
        return Err(exit_code);
    }
    Ok(())
}
