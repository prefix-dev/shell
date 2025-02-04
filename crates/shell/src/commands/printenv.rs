use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;

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
    let args = context.args.clone();

    let env_vars = context.state.env_vars();

    if args.is_empty() {
        // Print all environment variables
        let mut vars: Vec<_> = env_vars.iter().collect();
        vars.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (key, value) in vars {
            context
                .stdout
                .write_line(&format!("{}={}", key, value))
                .map_err(|_| 1)?;
        }
        Ok(())
    } else {
        // Print specified variables
        for name in args {
            match env_vars.get(&name) {
                Some(value) => {
                    context.stdout.write_line(value).map_err(|_| 1)?;
                }
                None => return Err(1),
            }
        }
        Ok(())
    }
}
