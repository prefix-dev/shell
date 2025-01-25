use std::{collections::HashMap, ffi::OsString, fs, rc::Rc};

use deno_task_shell::{EnvChange, ExecuteResult, ShellCommand, ShellCommandContext};
use futures::{future::LocalBoxFuture, FutureExt};

use uu_ls::uumain as uu_ls;

use crate::execute;

pub mod date;
pub mod set;
pub mod touch;
pub mod uname;
pub mod which;

pub use date::DateCommand;
pub use set::SetCommand;
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
        if context.args.len() != 1 {
            return Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)));
        }

        let script = context.args[0].clone();
        let script_file = context.state.cwd().join(script);
        match fs::read_to_string(&script_file) {
            Ok(content) => {
                let state = context.state.clone();
                async move {
                    execute::execute_inner(&content, script_file.display().to_string(), state)
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Could not source script: {:?}", script_file);
                            eprintln!("Error: {}", e);
                            ExecuteResult::from_exit_code(1)
                        })
                }
                .boxed_local()
            }
            Err(e) => {
                eprintln!("Could not read file: {:?} ({})", script_file, e);
                Box::pin(futures::future::ready(ExecuteResult::from_exit_code(1)))
            }
        }
    }
}
