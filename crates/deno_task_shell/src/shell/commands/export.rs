use super::{ShellCommand, ShellCommandContext};
use crate::shell::types::{EnvChange, ExecuteResult};
use futures::future::LocalBoxFuture;

fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let first_char = name.chars().next().unwrap();
    if first_char.is_ascii_digit() {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub struct ExportCommand;

impl ShellCommand for ExportCommand {
    fn execute(
        &self,
        mut context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let mut changes = Vec::new();

        for arg in context.args {
            if let Some(equals_index) = arg.find('=') {
                let arg_name = &arg[..equals_index];

                if !is_valid_identifier(arg_name) {
                    let _ = context.stderr.write_line(&format!(
                        "export: '{}': not a valid identifier",
                        arg_name
                    ));
                    return Box::pin(futures::future::ready(
                        ExecuteResult::Continue(1, Vec::new(), Vec::new()),
                    ));
                }

                let arg_value = &arg[equals_index + 1..];
                changes.push(EnvChange::SetEnvVar(
                    arg_name.to_string(),
                    arg_value.to_string(),
                ));
            }
        }

        Box::pin(futures::future::ready(ExecuteResult::Continue(
            0,
            changes,
            Vec::new(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_validation() {
        assert!(is_valid_identifier("valid"));
        assert!(is_valid_identifier("VALID_2"));
        assert!(!is_valid_identifier("2invalid"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("no spaces"));
    }
}
