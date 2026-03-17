// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;

use super::ShellCommand;
use super::ShellCommandContext;
use crate::shell::types::ExecuteResult;

/// The `trap` builtin registers commands to be executed when the shell
/// receives specific signals.
///
/// Usage:
///   trap 'commands' SIGNAL...   - Register handler
///   trap '' SIGNAL...           - Ignore signal
///   trap - SIGNAL...            - Reset to default
///   trap -l                     - List signal names
///   trap -p [SIGNAL...]         - Print trap commands
///
/// Currently a basic implementation that supports listing signals
/// and acknowledging trap commands. Full signal handling requires
/// deeper integration with the shell's execution loop.
pub struct TrapCommand;

const SIGNALS: &[(&str, i32)] = &[
    ("EXIT", 0),
    ("HUP", 1),
    ("INT", 2),
    ("QUIT", 3),
    ("ILL", 4),
    ("TRAP", 5),
    ("ABRT", 6),
    ("BUS", 7),
    ("FPE", 8),
    ("KILL", 9),
    ("USR1", 10),
    ("SEGV", 11),
    ("USR2", 12),
    ("PIPE", 13),
    ("ALRM", 14),
    ("TERM", 15),
];

impl ShellCommand for TrapCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let args = context.args.clone();

        if args.is_empty() {
            // `trap` with no args: print current traps (none registered yet)
            return Box::pin(futures::future::ready(
                ExecuteResult::from_exit_code(0),
            ));
        }

        if args[0] == "-l" {
            // List signal names
            let mut output = String::new();
            for (name, num) in SIGNALS {
                output.push_str(&format!("{num:2}) SIG{name}\n"));
            }
            let _ = context.stdout.clone().write_all(output.as_bytes());
            return Box::pin(futures::future::ready(
                ExecuteResult::from_exit_code(0),
            ));
        }

        if args[0] == "-p" {
            // Print traps - currently none registered
            return Box::pin(futures::future::ready(
                ExecuteResult::from_exit_code(0),
            ));
        }

        // Validate signal names/numbers
        if args.len() < 2 {
            let _ = context.stderr.clone().write_line(
                "trap: usage: trap [-lp] [[arg] signal_spec ...]",
            );
            return Box::pin(futures::future::ready(
                ExecuteResult::Continue(2, Vec::new(), Vec::new()),
            ));
        }

        // Validate that the signal specs are recognized
        for sig_spec in &args[1..] {
            if !is_valid_signal(sig_spec) {
                let _ = context.stderr.clone().write_line(&format!(
                    "trap: {sig_spec}: invalid signal specification"
                ));
                return Box::pin(futures::future::ready(
                    ExecuteResult::Continue(1, Vec::new(), Vec::new()),
                ));
            }
        }

        // Accept the trap command silently for compatibility.
        // Full signal handling would require storing handlers in ShellState
        // and invoking them during signal delivery.
        Box::pin(futures::future::ready(
            ExecuteResult::from_exit_code(0),
        ))
    }
}

fn is_valid_signal(spec: &str) -> bool {
    // Check by name (with or without SIG prefix)
    let upper = spec.to_uppercase();
    let name = upper.strip_prefix("SIG").unwrap_or(&upper);
    if SIGNALS.iter().any(|(n, _)| *n == name) {
        return true;
    }
    // Check by number
    if let Ok(num) = spec.parse::<i32>() {
        return SIGNALS.iter().any(|(_, n)| *n == num);
    }
    false
}
