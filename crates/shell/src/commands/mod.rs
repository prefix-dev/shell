use std::{collections::HashMap, ffi::OsString, fs, rc::Rc};

use deno_task_shell::{EnvChange, ExecuteResult, ShellCommand, ShellCommandContext};
use futures::{future::LocalBoxFuture, FutureExt};

use uu_ls::uumain as uu_ls;

pub mod command_cmd;
pub mod date;
pub mod printenv;
pub mod set;
pub mod time;
pub mod touch;
pub mod uname;
pub mod which;

pub use command_cmd::CommandCommand;
pub use command_cmd::TypeCommand;
pub use date::DateCommand;
pub use printenv::PrintEnvCommand;
pub use set::SetCommand;
pub use time::TimeCommand;
pub use touch::TouchCommand;
pub use uname::UnameCommand;
pub use which::WhichCommand;

pub struct LsCommand;

pub struct AliasCommand;

pub struct UnAliasCommand;

pub struct SourceCommand;

pub fn get_commands() -> HashMap<String, Rc<dyn ShellCommand>> {
    HashMap::from([
        ("ls".to_string(), Rc::new(LsCommand) as Rc<dyn ShellCommand>),
        (
            "alias".to_string(),
            Rc::new(AliasCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "unalias".to_string(),
            Rc::new(UnAliasCommand) as Rc<dyn ShellCommand>,
        ),
        (
            ".".to_string(),
            Rc::new(SourceCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "source".to_string(),
            Rc::new(SourceCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "which".to_string(),
            Rc::new(WhichCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "uname".to_string(),
            Rc::new(UnameCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "touch".to_string(),
            Rc::new(TouchCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "date".to_string(),
            Rc::new(DateCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "set".to_string(),
            Rc::new(SetCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "printenv".to_string(),
            Rc::new(PrintEnvCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "clear".to_string(),
            Rc::new(ClearCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "time".to_string(),
            Rc::new(TimeCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "command".to_string(),
            Rc::new(CommandCommand) as Rc<dyn ShellCommand>,
        ),
        (
            "type".to_string(),
            Rc::new(TypeCommand) as Rc<dyn ShellCommand>,
        ),
    ])
}

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
        async move {
            if context.args.is_empty() {
                let _ = context
                    .stderr
                    .clone()
                    .write_line("source: filename argument required");
                return ExecuteResult::Continue(2, Vec::new(), Vec::new());
            }

            let filename = &context.args[0];
            let script_file = if std::path::Path::new(filename).is_absolute() {
                std::path::PathBuf::from(filename)
            } else {
                context.state.cwd().join(filename)
            };

            let contents = match fs::read_to_string(&script_file) {
                Ok(c) => c,
                Err(err) => {
                    let _ = context
                        .stderr
                        .clone()
                        .write_line(&format!("source: {}: {err}", script_file.display()));
                    return ExecuteResult::Continue(1, Vec::new(), Vec::new());
                }
            };

            let parsed = match deno_task_shell::parser::parse(&contents) {
                Ok(list) => list,
                Err(err) => {
                    let _ = context.stderr.clone().write_line(&format!("source: {err}"));
                    return ExecuteResult::Continue(2, Vec::new(), Vec::new());
                }
            };

            deno_task_shell::execute_sequential_list(
                parsed,
                context.state,
                context.stdin,
                context.stdout,
                context.stderr,
                deno_task_shell::AsyncCommandBehavior::Wait,
            )
            .await
        }
        .boxed_local()
    }
}

pub struct ClearCommand;

impl ShellCommand for ClearCommand {
    fn execute(&self, _context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(async move {
            // ANSI escape sequence to clear screen and move cursor to top
            print!("\x1B[2J\x1B[1;1H");
            // Ensure output is flushed
            let _ = std::io::Write::flush(&mut std::io::stdout());
            ExecuteResult::Continue(0, vec![], vec![])
        })
    }
}
