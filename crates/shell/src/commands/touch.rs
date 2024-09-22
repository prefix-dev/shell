use std::{ffi::OsString, path::Path};

use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;
use uu_touch::uumain as uu_touch;

pub struct TouchCommand;

impl ShellCommand for TouchCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(match execute_touch(&mut context) {
            Ok(_) => ExecuteResult::from_exit_code(0),
            Err(exit_code) => ExecuteResult::from_exit_code(exit_code),
        }))
    }
}

fn execute_touch(context: &mut ShellCommandContext) -> Result<(), i32> {
    let mut args: Vec<OsString> = vec![OsString::from("touch")];

    context
        .args
        .iter()
        .for_each(|arg| args.push(OsString::from(arg)));

    let mut new_args = Vec::new();
    let mut skip_next = false;

    for (index, arg) in args[1..].iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg.to_str().map_or(false, |s| s == "-t" || s == "-d") && index + 1 < args[1..].len() {
            new_args.push(arg.clone());
            new_args.push(args[index + 2].clone());
            skip_next = true;
        } else if !arg.to_str().map_or(false, |s| s.starts_with('-')) {
            new_args.push(if Path::new(arg).is_absolute() {
                arg.clone()
            } else {
                context.state.cwd().join(arg).into_os_string()
            });
        } else {
            new_args.push(arg.clone());
        }
    }

    args.splice(1.., new_args);

    let exit_code = uu_touch(args.into_iter());
    if exit_code != 0 {
        return Err(exit_code);
    }
    Ok(())
}
