// Copyright 2018-2024 the Deno authors. MIT license.

pub use command::ResolveCommandPathError;
pub use commands::ExecutableCommand;
pub use commands::ExecuteCommandArgsContext;
pub use commands::ShellCommand;
pub use commands::ShellCommandContext;
pub use execute::execute;
pub use execute::{
  execute_sequential_list, execute_with_pipes, AsyncCommandBehavior,
};
pub use types::pipe;
pub use types::EnvChange;
pub use types::ExecuteResult;
pub use types::FutureExecuteResult;
pub use types::ShellPipeReader;
pub use types::ShellPipeWriter;
pub use types::ShellState;
pub use types::ShellOptions;

pub use commands::parse_arg_kinds;
pub use commands::ArgKind;

pub mod fs_util;

mod command;
mod commands;
mod execute;
mod types;
