// Runs the `shell` binary on a script with arguments, to test how the
// command line becomes the script's positional parameters.

use std::path::PathBuf;
use std::process::Command;

fn run_script(name: &str, script: &str, args: &[&str]) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let path = dir.join(name);
    std::fs::write(&path, script).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_shell"))
        .arg("--norc")
        .arg(&path)
        .args(args)
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn script_positional_parameters() {
    let script = r#"echo "count=$#"
echo "0=$0"
echo "1=$1"
echo "2=$2"
echo "all=$@"
echo "star=$*"
"#;
    let expected_0 = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("positional.sh")
        .display()
        .to_string();
    assert_eq!(
        run_script("positional.sh", script, &["foo", "b c"]),
        format!("count=2\n0={expected_0}\n1=foo\n2=b c\nall=foo b c\nstar=foo b c\n")
    );
}

#[test]
fn script_without_arguments() {
    let script = r#"echo "count=$#"
if [ "$#" -gt 0 ]; then
    shift
fi
echo "all=[$@]"
"#;
    assert_eq!(run_script("no_args.sh", script, &[]), "count=0\nall=[]\n");
}

#[test]
fn script_forwards_arguments() {
    // "$@" keeps each script argument as a separate word, also after shift
    let script = r#"build_dir="${1:-.}"
if [ "$#" -gt 0 ]; then
    shift
fi
echo "build_dir=$build_dir"
for a in -S . "$@"; do
    echo "[$a]"
done
"#;
    assert_eq!(
        run_script("forward.sh", script, &["build", "-DA=1", "-DB=x y"]),
        "build_dir=build\n[-S]\n[.]\n[-DA=1]\n[-DB=x y]\n"
    );
    assert_eq!(
        run_script("forward_none.sh", script, &[]),
        "build_dir=.\n[-S]\n[.]\n"
    );
}
