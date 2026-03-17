// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;

/// The `local` builtin declares variables with local scope.
/// Since function support is not yet implemented, this behaves like
/// setting a shell variable (not exported). This matches bash behavior
/// where `local` outside a function generates a warning but still works.
pub struct LocalCommand;

impl ShellCommand for LocalCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let mut changes = Vec::new();

        for arg in &context.args {
            if let Some(equals_index) = arg.find('=') {
                let name = &arg[..equals_index];
                let value = &arg[equals_index + 1..];

                if !is_valid_var_name(name) {
                    let _ = context.stderr.clone().write_line(&format!(
                        "local: `{name}': not a valid identifier"
                    ));
                    return Box::pin(futures::future::ready(
                        ExecuteResult::Continue(1, Vec::new(), Vec::new()),
                    ));
                }

                changes.push(EnvChange::SetShellVar(
                    name.to_string(),
                    value.to_string(),
                ));
            } else {
                // `local VAR` without assignment - declare it with empty value
                if !is_valid_var_name(arg) {
                    let _ = context.stderr.clone().write_line(&format!(
                        "local: `{arg}': not a valid identifier"
                    ));
                    return Box::pin(futures::future::ready(
                        ExecuteResult::Continue(1, Vec::new(), Vec::new()),
                    ));
                }

                // Only set if not already defined
                if context.state.get_var(arg).is_none() {
                    changes.push(EnvChange::SetShellVar(
                        arg.to_string(),
                        String::new(),
                    ));
                }
            }
        }

        Box::pin(futures::future::ready(ExecuteResult::Continue(
            0, changes, Vec::new(),
        )))
    }
}

fn is_valid_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
