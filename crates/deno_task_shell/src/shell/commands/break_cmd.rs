// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;
use miette::bail;
use miette::Result;

use crate::shell::types::ExecuteResult;

use super::args::parse_arg_kinds;
use super::args::ArgKind;
use super::ShellCommand;
use super::ShellCommandContext;

pub struct BreakCommand;

impl ShellCommand for BreakCommand {
    fn execute(
        &self,
        mut context: ShellCommandContext,
    ) -> LocalBoxFuture<'static, ExecuteResult> {
        let result = match execute_break(context.args) {
            Ok(code) => ExecuteResult::Break(code, Vec::new(), Vec::new()),
            Err(err) => {
                context.stderr.write_line(&format!("break: {err}")).unwrap();
                ExecuteResult::Continue(1, Vec::new(), Vec::new())
            }
        };
        Box::pin(futures::future::ready(result))
    }
}

fn execute_break(args: Vec<String>) -> Result<i32> {
    let _n = parse_args(args)?;
    // For now, we only support breaking out of the innermost loop
    // TODO: Support breaking out of n levels of loops
    Ok(0)
}

fn parse_args(args: Vec<String>) -> Result<i32> {
    let args = parse_arg_kinds(&args);
    let mut paths = Vec::new();
    for arg in args {
        match arg {
            ArgKind::Arg(arg) => {
                paths.push(arg);
            }
            _ => arg.bail_unsupported()?,
        }
    }

    match paths.len() {
        0 => Ok(1),
        1 => {
            let arg = paths.remove(0).to_string();
            match arg.parse::<i32>() {
                Ok(value) if value > 0 => Ok(value),
                Ok(_) => bail!("loop count out of range"),
                Err(_) => bail!("numeric argument required"),
            }
        }
        _ => {
            bail!("too many arguments")
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_args() {
        assert_eq!(parse_args(vec![]).unwrap(), 1);
        assert_eq!(parse_args(vec!["1".to_string()]).unwrap(), 1);
        assert_eq!(parse_args(vec!["2".to_string()]).unwrap(), 2);
        assert_eq!(
            parse_args(vec!["0".to_string()]).err().unwrap().to_string(),
            "loop count out of range"
        );
        assert_eq!(
            parse_args(vec!["-1".to_string()])
                .err()
                .unwrap()
                .to_string(),
            "loop count out of range"
        );
        assert_eq!(
            parse_args(vec!["test".to_string()])
                .err()
                .unwrap()
                .to_string(),
            "numeric argument required"
        );
        assert_eq!(
            parse_args(vec!["1".to_string(), "2".to_string()])
                .err()
                .unwrap()
                .to_string(),
            "too many arguments"
        );
    }

    #[test]
    fn executes_break() {
        assert_eq!(execute_break(vec![]).unwrap(), 0);
        assert_eq!(execute_break(vec!["1".to_string()]).unwrap(), 0);
    }
}
