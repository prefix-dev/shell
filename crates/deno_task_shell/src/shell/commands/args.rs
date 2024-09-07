// Copyright 2018-2024 the Deno authors. MIT license.

use anyhow::{bail, Context, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum ArgKind<'a> {
  ShortFlag(char),
  LongFlag(&'a str),
  Arg(&'a str),
}

impl<'a> ArgKind<'a> {
  pub fn bail_unsupported(&self) -> anyhow::Result<()> {
    match self {
      ArgKind::Arg(arg) => {
        bail!("unsupported argument: {}", arg)
      }
      ArgKind::LongFlag(name) => {
        bail!("unsupported flag: --{}", name)
      }
      ArgKind::ShortFlag(name) => {
        bail!("unsupported flag: -{}", name)
      }
    }
  }
}

pub fn parse_arg_kinds(
  flags: &mut [String],
) -> Result<Vec<ArgKind>, anyhow::Error> {
  let mut result = Vec::new();
  let mut had_dash_dash = false;
  let home_str = dirs::home_dir()
    .context("Couldn't get home directory")?
    .to_string_lossy()
    .into_owned();
  for arg in flags.iter_mut() {
    if had_dash_dash {
      let arg_clone = arg.clone();
      arg.replace_range(.., &arg_clone.replace('~', &home_str));
      result.push(ArgKind::Arg(arg));
    } else if arg == "-" {
      result.push(ArgKind::Arg("-"));
    } else if arg == "--" {
      had_dash_dash = true;
    } else if arg.starts_with("--") {
      result.push(ArgKind::LongFlag(arg.strip_prefix("--").unwrap()));
    } else if arg.starts_with('-') {
      if arg.parse::<f64>().is_ok() {
        result.push(ArgKind::Arg(arg));
      } else {
        for c in arg.strip_prefix('-').unwrap().chars() {
          result.push(ArgKind::ShortFlag(c));
        }
      }
    } else {
      let arg_clone = arg.clone();
      arg.replace_range(.., &arg_clone.replace('~', &home_str));
      result.push(ArgKind::Arg(arg));
    }
  }
  Ok(result)
}

#[cfg(test)]
mod test {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn parses() {
    let mut data = vec![
      "-f".to_string(),
      "-ab".to_string(),
      "--force".to_string(),
      "testing".to_string(),
      "other".to_string(),
      "-1".to_string(),
      "-6.4".to_string(),
      "--".to_string(),
      "--test".to_string(),
      "-t".to_string(),
    ];
    let args = parse_arg_kinds(&mut data);
    assert!(args.is_ok());
    assert_eq!(
      args.unwrap(),
      vec![
        ArgKind::ShortFlag('f'),
        ArgKind::ShortFlag('a'),
        ArgKind::ShortFlag('b'),
        ArgKind::LongFlag("force"),
        ArgKind::Arg("testing"),
        ArgKind::Arg("other"),
        ArgKind::Arg("-1"),
        ArgKind::Arg("-6.4"),
        ArgKind::Arg("--test"),
        ArgKind::Arg("-t"),
      ]
    )
  }
}
