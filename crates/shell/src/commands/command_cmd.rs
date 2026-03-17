use deno_task_shell::{ExecuteResult, ShellCommand, ShellCommandContext};
use futures::future::LocalBoxFuture;
/// The `command` builtin - runs a command bypassing aliases, or with -v
/// prints the path/type of a command.
pub struct CommandCommand;

impl ShellCommand for CommandCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        let args = context.args.clone();

        // Check for -v flag (command -v name)
        if args.first().map(|s| s.as_str()) == Some("-v") {
            return Box::pin(futures::future::ready(execute_command_v(
                &args[1..],
                context,
            )));
        }

        // Without -v, `command name args...` runs the command bypassing aliases.
        // We pass through to the command execution, stripping aliases.
        if args.is_empty() {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(0)));
        }

        // Execute the command directly (bypassing alias resolution)
        let execute_command_args = context.execute_command_args;
        let state = context.state;
        let stdin = context.stdin;
        let stdout = context.stdout;
        let stderr = context.stderr;

        execute_command_args(deno_task_shell::ExecuteCommandArgsContext {
            args,
            state,
            stdin,
            stdout,
            stderr,
        })
    }
}

fn execute_command_v(names: &[String], mut context: ShellCommandContext) -> ExecuteResult {
    if names.is_empty() {
        return ExecuteResult::Continue(1, Vec::new(), Vec::new());
    }

    let mut exit_code = 0;

    for name in names {
        // Check builtins first
        if context.state.resolve_custom_command(name).is_some() {
            let _ = context.stdout.write_line(name);
            continue;
        }

        // Check PATH
        if let Some(path) = context.state.env_vars().get("PATH") {
            let path = std::ffi::OsString::from(path);
            let which_result = which::which_in_global(name, Some(path))
                .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath));

            if let Ok(p) = which_result {
                let _ = context.stdout.write_line(&p.to_string_lossy());
                continue;
            }
        }

        // Not found
        exit_code = 1;
    }

    ExecuteResult::Continue(exit_code, Vec::new(), Vec::new())
}

/// The `type` builtin - describes how a command name would be interpreted.
pub struct TypeCommand;

impl ShellCommand for TypeCommand {
    fn execute(&self, mut context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        let args = context.args.clone();
        Box::pin(futures::future::ready(execute_type(&args, &mut context)))
    }
}

fn execute_type(args: &[String], context: &mut ShellCommandContext) -> ExecuteResult {
    if args.is_empty() {
        return ExecuteResult::Continue(1, Vec::new(), Vec::new());
    }

    let mut exit_code = 0;

    for name in args {
        // Check aliases
        if let Some(alias) = context.state.alias_map().get(name) {
            let _ =
                context
                    .stdout
                    .write_line(&format!("{} is aliased to `{}`", name, alias.join(" ")));
            continue;
        }

        // Check builtins
        if context.state.resolve_custom_command(name).is_some() {
            let _ = context
                .stdout
                .write_line(&format!("{} is a shell builtin", name));
            continue;
        }

        // Check PATH
        if let Some(path) = context.state.env_vars().get("PATH") {
            let path = std::ffi::OsString::from(path);
            let which_result = which::which_in_global(name, Some(path))
                .and_then(|mut i| i.next().ok_or(which::Error::CannotFindBinaryPath));

            if let Ok(p) = which_result {
                let _ = context
                    .stdout
                    .write_line(&format!("{} is {}", name, p.to_string_lossy()));
                continue;
            }
        }

        let _ = context
            .stderr
            .write_line(&format!("type: {}: not found", name));
        exit_code = 1;
    }

    ExecuteResult::Continue(exit_code, Vec::new(), Vec::new())
}
