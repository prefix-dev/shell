use futures::future::LocalBoxFuture;

use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;

use super::ShellCommand;
use super::ShellCommandContext;

pub struct ReadCommand;

impl ShellCommand for ReadCommand {
    fn execute(
        &self,
        context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        Box::pin(async move { execute_read(context) })
    }
}

fn execute_read(mut context: ShellCommandContext) -> ExecuteResult {
    let mut raw_mode = false;
    let mut prompt = String::new();
    let mut var_names: Vec<String> = Vec::new();

    // Parse arguments
    let mut i = 0;
    while i < context.args.len() {
        match context.args[i].as_str() {
            "-r" => raw_mode = true,
            "-p" => {
                i += 1;
                if i < context.args.len() {
                    prompt = context.args[i].clone();
                }
            }
            arg if arg.starts_with('-') => {
                // Ignore unknown flags for forward compatibility
            }
            _ => {
                var_names.push(context.args[i].clone());
            }
        }
        i += 1;
    }

    // Default variable name is REPLY
    if var_names.is_empty() {
        var_names.push("REPLY".to_string());
    }

    // Write prompt if specified
    if !prompt.is_empty() {
        let _ = context.stderr.write_all(prompt.as_bytes());
    }

    // Read a line from stdin
    let mut line = String::new();
    let mut buf = [0u8; 1];
    loop {
        match context.stdin.read(&mut buf) {
            Ok(0) => {
                // EOF
                if line.is_empty() {
                    return ExecuteResult::Continue(1, Vec::new(), Vec::new());
                }
                break;
            }
            Ok(_) => {
                if buf[0] == b'\n' {
                    break;
                }
                line.push(buf[0] as char);
            }
            Err(_) => {
                return ExecuteResult::Continue(1, Vec::new(), Vec::new());
            }
        }
    }

    // Handle backslash escapes unless -r is specified
    if !raw_mode {
        line = process_backslashes(&line);
    }

    // Remove trailing carriage return (for Windows line endings)
    if line.ends_with('\r') {
        line.pop();
    }

    // Split the line into fields and assign to variables
    let mut changes = Vec::new();
    if var_names.len() == 1 {
        // Single variable gets the entire line (trimmed of leading/trailing whitespace)
        let value = line.trim().to_string();
        changes.push(EnvChange::SetShellVar(var_names[0].clone(), value));
    } else {
        // Multiple variables: split by whitespace
        let trimmed = line.trim_start();
        let parts: Vec<&str> = trimmed
            .splitn(var_names.len(), char::is_whitespace)
            .collect();

        for (idx, var_name) in var_names.iter().enumerate() {
            let value = if idx < parts.len() {
                parts[idx].to_string()
            } else {
                String::new()
            };
            changes.push(EnvChange::SetShellVar(var_name.clone(), value));
        }
    }

    ExecuteResult::Continue(0, changes, Vec::new())
}

fn process_backslashes(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                // Backslash-newline is line continuation (skip both)
                if next == '\n' {
                    chars.next();
                    continue;
                }
                // Otherwise, the backslash escapes the next character
                result.push(next);
                chars.next();
            } else {
                // Trailing backslash
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_backslashes() {
        assert_eq!(process_backslashes("hello"), "hello");
        assert_eq!(process_backslashes("he\\nllo"), "henllo");
        assert_eq!(process_backslashes("he\\\\llo"), "he\\llo");
    }
}
