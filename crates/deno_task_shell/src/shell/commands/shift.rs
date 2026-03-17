// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;

/// The `shift` builtin shifts positional parameters ($1, $2, ...) left by N (default 1).
/// Since we store positional params as shell vars "1", "2", etc., we re-number them.
pub struct ShiftCommand;

impl ShellCommand for ShiftCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let n: usize = if context.args.is_empty() {
            1
        } else {
            match context.args[0].parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    let _ = context.stderr.clone().write_line(&format!(
                        "shift: {}: numeric argument required",
                        context.args[0]
                    ));
                    return Box::pin(futures::future::ready(
                        ExecuteResult::Continue(1, Vec::new(), Vec::new()),
                    ));
                }
            }
        };

        // Collect current positional parameters
        let mut positional: Vec<String> = Vec::new();
        let mut i = 1;
        loop {
            match context.state.get_var(&i.to_string()) {
                Some(val) => {
                    positional.push(val.clone());
                    i += 1;
                }
                None => break,
            }
        }

        let total = positional.len();
        if n > total {
            let _ = context.stderr.clone().write_line(&format!(
                "shift: {n}: shift count out of range"
            ));
            return Box::pin(futures::future::ready(
                ExecuteResult::Continue(1, Vec::new(), Vec::new()),
            ));
        }

        let mut changes = Vec::new();

        // Set the new positional parameters (shifted)
        let remaining = &positional[n..];
        for (idx, val) in remaining.iter().enumerate() {
            changes.push(EnvChange::SetShellVar(
                (idx + 1).to_string(),
                val.clone(),
            ));
        }

        // Unset the old positions that are now beyond the new count
        for idx in (remaining.len() + 1)..=total {
            changes.push(EnvChange::UnsetVar(idx.to_string()));
        }

        // Update $# (parameter count)
        changes.push(EnvChange::SetShellVar(
            "#".to_string(),
            remaining.len().to_string(),
        ));

        Box::pin(futures::future::ready(ExecuteResult::Continue(
            0, changes, Vec::new(),
        )))
    }
}
