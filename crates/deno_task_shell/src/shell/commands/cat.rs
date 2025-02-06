// Copyright 2018-2024 the Deno authors. MIT license.

use futures::future::LocalBoxFuture;
use futures::FutureExt as _;
use miette::IntoDiagnostic;
use miette::Result;
use tokio::fs::File;
use tokio::io::AsyncReadExt as _;
use std::io::IsTerminal;
use std::path::Path;

use crate::shell::commands::execute_with_cancellation;
use crate::shell::types::ExecuteResult;
use crate::ShellPipeReader;
use crate::ShellPipeWriter;

use super::args::parse_arg_kinds;
use super::args::ArgKind;
use super::ShellCommand;
use super::ShellCommandContext;

pub struct CatCommand;

impl ShellCommand for CatCommand {
    fn execute(&self, context: ShellCommandContext) -> LocalBoxFuture<'static, ExecuteResult> {
        async move {
            execute_with_cancellation!(
                cat_command(
                    context.state.cwd(),
                    context.args,
                    context.stdin,
                    context.stdout,
                    context.stderr
                ),
                context.state.token()
            )
        }
        .boxed_local()
    }
}

async fn cat_command(
    cwd: &Path,
    args: Vec<String>,
    stdin: ShellPipeReader,
    mut stdout: ShellPipeWriter,
    mut stderr: ShellPipeWriter,
) -> ExecuteResult {
    match execute_cat(cwd, args, stdin, &mut stdout, &mut stderr).await {
        Ok(()) => ExecuteResult::Continue(0, Vec::new(), Vec::new()),
        Err(err) => {
            let _ = stderr.write_line(&format!("cat: {err}"));
            ExecuteResult::Continue(1, Vec::new(), Vec::new())
        }
    }
}

async fn execute_cat(
    cwd: &Path,
    args: Vec<String>,
    stdin: ShellPipeReader,
    stdout: &mut ShellPipeWriter,
    stderr: &mut ShellPipeWriter,
) -> Result<()> {
    let flags = parse_args(args)?;
    let mut buf = vec![0; 1024];

    for path in flags.paths {
        if path == "-" {
            stdin.clone().pipe_to_sender(stdout.clone())?;
        } else {
            match File::open(cwd.join(&path)).await {
                Ok(mut file) => {
                    let mut new_line = true;
                    loop {
                        let size = file.read(&mut buf).await.into_diagnostic()?;
                        if size == 0 {
                            if let ShellPipeWriter::Stdout = stdout {
                                if !new_line && std::io::stdout().is_terminal() {
                                    stdout.write_all(b"%\n")?;
                                }
                            }
                            break;
                        }
                        stdout.write_all(&buf[..size])?;
                        new_line = buf[size - 1] == b'\n';
                    }
                }
                Err(err) => {
                    stderr.write_line(&format!("cat: {path}: {err}"))?;
                    miette::bail!("failed to open file: {path}");
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct CatFlags {
    paths: Vec<String>,
}

fn parse_args(args: Vec<String>) -> Result<CatFlags> {
    let mut paths = Vec::new();
    for arg in parse_arg_kinds(&args) {
        match arg {
            ArgKind::Arg(file_name) => {
                paths.push(file_name.to_string());
            }
            // for now, we don't support any arguments
            _ => arg.bail_unsupported()?,
        }
    }

    if paths.is_empty() {
        paths.push("-".to_string());
    }

    Ok(CatFlags { paths })
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_args() {
        assert_eq!(
            parse_args(vec![]).unwrap(),
            CatFlags {
                paths: vec!["-".to_string()]
            }
        );
        assert_eq!(
            parse_args(vec!["path".to_string()]).unwrap(),
            CatFlags {
                paths: vec!["path".to_string()]
            }
        );
        assert_eq!(
            parse_args(vec!["path".to_string(), "-".to_string()]).unwrap(),
            CatFlags {
                paths: vec!["path".to_string(), "-".to_string()]
            }
        );
        assert_eq!(
            parse_args(vec!["path".to_string(), "other-path".to_string()])
                .unwrap(),
            CatFlags {
                paths: vec!["path".to_string(), "other-path".to_string()]
            }
        );
        assert_eq!(
            parse_args(vec!["--flag".to_string()])
                .err()
                .unwrap()
                .to_string(),
            "unsupported flag: --flag"
        );
        assert_eq!(
            parse_args(vec!["-t".to_string()])
                .err()
                .unwrap()
                .to_string(),
            "unsupported flag: -t"
        );
    }
}
