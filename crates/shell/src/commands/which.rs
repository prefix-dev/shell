use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

pub struct WhichCommand;

impl ShellCommand for WhichCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(execute_which(context)))
    }
}

fn execute_which(mut context: ShellCommandContext) -> ExecuteResult {
    if context.args.len() != 1 {
        context.stderr.write_line("Expected one argument.").unwrap();
        return ExecuteResult::from_exit_code(1);
    }
    let arg = &context.args[0];

    if let Some(alias) = context.state.alias_map().get(arg) {
        context
            .stdout
            .write_line(&format!("alias: \"{}\"", alias.join(" ")))
            .unwrap();
        return ExecuteResult::from_exit_code(0);
    }

    if context.state.resolve_custom_command(arg).is_some() {
        context.stdout.write_line("<builtin function>").unwrap();
        return ExecuteResult::from_exit_code(0);
    }

    if let Some(path) = context.state.env_vars().get("PATH") {
        let path = std::ffi::OsString::from(path);
        let which_result = which::which_in_global(arg, Some(path))
            .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath));

        if let Ok(p) = which_result {
            context.stdout.write_line(&p.to_string_lossy()).unwrap();
            return ExecuteResult::from_exit_code(0);
        }
    }

    context
        .stderr
        .write_line(&format!("{} not found", arg))
        .unwrap();
    ExecuteResult::from_exit_code(1)
}
