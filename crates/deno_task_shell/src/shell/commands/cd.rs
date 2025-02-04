// Copyright 2018-2024 the Deno authors. MIT license.

use std::path::Path;
use std::path::PathBuf;

use futures::future::LocalBoxFuture;
use miette::bail;
use miette::Result;
use path_dedot::ParseDot;

use crate::shell::fs_util;
use crate::shell::types::EnvChange;
use crate::shell::types::ExecuteResult;

use super::ShellCommand;
use super::ShellCommandContext;

pub struct CdCommand;

fn resolve_directory(
  dir: &str,
  cwd: &Path,
  prev_cwd: Option<&PathBuf>,
) -> Result<PathBuf> {
  match dir {
    "-" => Ok(
      prev_cwd
        .ok_or_else(|| miette::miette!("No previous directory"))?
        .to_path_buf(),
    ),
    "~" => dirs::home_dir()
      .ok_or_else(|| miette::miette!("Home directory not found")),
    _ if dir.starts_with("~/") => {
      let home = dirs::home_dir()
        .ok_or_else(|| miette::miette!("Home directory not found"))?;
      Ok(home.join(&dir[2..]))
    }
    _ => Ok(cwd.join(dir)),
  }
}

fn execute_cd(
  cwd: &Path,
  prev_cwd: Option<&PathBuf>,
  args: Vec<String>,
) -> Result<PathBuf> {
  let path = if args.is_empty() {
    "~".to_string()
  } else if args.len() > 1 {
    bail!("too many arguments")
  } else {
    args[0].clone()
  };

  let new_dir = resolve_directory(&path, cwd, prev_cwd)?;
  let new_dir = new_dir
    .parse_dot()
    .map(|p| p.to_path_buf())
    .unwrap_or_else(|_| fs_util::canonicalize_path(&new_dir).unwrap());

  if !new_dir.is_dir() {
    bail!("{}: Not a directory", path)
  }
  Ok(new_dir)
}

impl ShellCommand for CdCommand {
  fn execute(
    &self,
    mut context: ShellCommandContext,
  ) -> LocalBoxFuture<'static, ExecuteResult> {
    Box::pin(async move {
      match execute_cd(
        context.state.cwd(),
        context.state.previous_cwd(),
        context.args,
      ) {
        Ok(new_dir) => {
          ExecuteResult::Continue(0, vec![EnvChange::Cd(new_dir)], Vec::new())
        }
        Err(err) => {
          let _ = context.stderr.write_line(&format!("cd: {err}"));
          ExecuteResult::Continue(1, Vec::new(), Vec::new())
        }
      }
    })
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use std::fs;
  use tempfile::tempdir;

  #[test]
  fn test_cd_previous() {
    let dir = tempdir().unwrap();
    let dir_path = fs_util::canonicalize_path(dir.path()).unwrap();
    let sub_dir = dir_path.join("sub");
    std::fs::create_dir(&sub_dir).unwrap();

    // Test cd -
    let sub_dir = Some(sub_dir);
    let result =
      execute_cd(&dir_path, sub_dir.as_ref(), vec!["-".to_string()]).unwrap();
    assert_eq!(Some(result), sub_dir);
  }

  #[test]
  fn test_directory_navigation() {
    let dir = tempdir().unwrap();
    let dir_path = fs_util::canonicalize_path(dir.path()).unwrap();
    let prev_dir = dir_path.join("prev");
    fs::create_dir(&prev_dir).unwrap();
    let prev_dir = Some(prev_dir);
    // Test basic navigation
    let result = execute_cd(&dir_path, prev_dir.as_ref(), vec![]).unwrap();
    assert_eq!(result, dirs::home_dir().unwrap());

    // Test cd -
    let result =
      execute_cd(&dir_path, prev_dir.as_ref(), vec!["-".to_string()]).unwrap();
    assert_eq!(Some(result), prev_dir);

    // Test home expansion
    let result =
      execute_cd(&dir_path, prev_dir.as_ref(), vec!["~".to_string()]).unwrap();
    assert_eq!(result, dirs::home_dir().unwrap());

    // Test non-existent directory
    let err = execute_cd(
      &dir_path,
      prev_dir.as_ref(),
      vec!["non-existent".to_string()],
    )
    .unwrap_err();
    assert!(err.to_string().contains("Not a directory"));

    // Test file instead of directory
    fs::write(dir_path.join("file.txt"), "").unwrap();
    let err =
      execute_cd(&dir_path, prev_dir.as_ref(), vec!["file.txt".to_string()])
        .unwrap_err();
    assert!(err.to_string().contains("Not a directory"));

    // Test too many arguments
    let err = execute_cd(
      &dir_path,
      prev_dir.as_ref(),
      vec!["a".to_string(), "b".to_string()],
    )
    .unwrap_err();
    assert!(err.to_string().contains("too many arguments"));
  }

  #[test]
  fn test_path_resolution() {
    let dir = tempdir().unwrap();
    let dir_path = fs_util::canonicalize_path(dir.path()).unwrap();
    let prev_dir = Some(dir_path.clone());
    // Test nested directory
    fs::create_dir_all(dir_path.join("a/b/c")).unwrap();
    let result =
      execute_cd(&dir_path, prev_dir.as_ref(), vec!["a/b/c".to_string()])
        .unwrap();
    assert_eq!(result, dir_path.join("a/b/c"));

    // Test dot navigation
    let result = execute_cd(
      &dir_path.join("a/b/c"),
      prev_dir.as_ref(),
      vec!["../..".to_string()],
    )
    .unwrap();
    assert_eq!(result, dir_path.join("a"));
  }
}
