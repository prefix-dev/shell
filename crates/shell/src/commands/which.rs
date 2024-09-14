use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

pub struct WhichCommand;

impl ShellCommand for WhichCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        if context.args.len() != 1 {
            context.stderr.write_line("Expected one argument.").unwrap();
        }
        let arg = &context.args[0];

        if let Some(alias) = context.state.alias_map().get(arg) {
            context
                .stdout
                .write_line(&format!("alias: \"{}\"", alias.join(" ")))
                .unwrap();
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(0)));
        }

        if context.state.resolve_custom_command(arg).is_some() {
            context.stdout.write_line("<builtin function>").unwrap();
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(0)));
        }

        if let Ok(p) = which::which(arg) {
            context.stdout.write_line(&p.to_string_lossy()).unwrap();
        }

        Box::pin(futures::future::ready(ExecuteResult::from_exit_code(0)))
    }
}
