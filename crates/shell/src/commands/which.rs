use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

pub struct WhichCommand;

impl ShellCommand for WhichCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(futures::future::ready(match execute_which(&mut context) {
            Ok(_) => ExecuteResult::from_exit_code(0),
            Err(exit_code) => ExecuteResult::from_exit_code(exit_code),
        }))
    }
}

fn execute_which(context: &mut ShellCommandContext) -> Result<(), i32> {
    if context.args.len() != 1 {
        context.stderr.write_line("Expected one argument").ok();
        return Err(1);
    }

    let arg = &context.args[0];

    if let Some(alias) = context.state.alias_map().get(arg) {
        context
            .stdout
            .write_line(&format!("alias: \"{}\"", alias.join(" ")))
            .ok();
        return Ok(());
    }

    if context.state.get_function(arg).is_some() {
        context.stdout.write_line("<user function>").ok();
        return Ok(());
    }

    if context.state.resolve_custom_command(arg).is_some() {
        context.stdout.write_line("<builtin function>").ok();
        return Ok(());
    }

    if let Some(path) = context.state.env_vars().get("PATH") {
        let path = std::ffi::OsString::from(path);
        let which_result = which::which_in_global(arg, Some(path))
            .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath));

        if let Ok(p) = which_result {
            context.stdout.write_line(&p.to_string_lossy()).ok();
            return Ok(());
        }
    }

    context
        .stderr
        .write_line(&format!("{} not found", arg))
        .ok();

    Err(1)
}
