// Copyright 2018-2024 the Deno authors. MIT license.
#![cfg(test)]

mod test_builder;
mod test_runner;

use deno_task_shell::ExecuteResult;
use futures::FutureExt;
use test_builder::TestBuilder;

const FOLDER_SEPARATOR: char = if cfg!(windows) { '\\' } else { '/' };

#[tokio::test]
async fn commands() {
    TestBuilder::new()
        .command("echo 1")
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo 1 2   3")
        .assert_stdout("1 2 3\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "1 2   3""#)
        .assert_stdout("1 2   3\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r"echo 1 2\ \ \ 3")
        .assert_stdout("1 2   3\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "1 2\ \ \ 3""#)
        .assert_stdout("1 2\\ \\ \\ 3\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo test$(echo "1    2")"#)
        .assert_stdout("test1 2\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"TEST="1   2" ; echo $TEST"#)
        .assert_stdout("1   2\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"TEST="1  2 " ; echo "${TEST:-}""#)
        .assert_stdout("1  2 \n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#""echo" "1""#)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#""echo" "*""#)
        .assert_stdout("*\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo test-dashes")
        .assert_stdout("test-dashes\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1; echo "$FOO""#)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo 'a/b'/c")
        .assert_stdout("a/b/c\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo 'a/b'ctest\"te  st\"'asdf'")
        .assert_stdout("a/bctestte  stasdf\n")
        .run()
        .await;

    TestBuilder::new()
    .command("echo --test=\"2\" --test='2' test\"TEST\" TEST'test'TEST 'test''test' test'test'\"test\" \"test\"\"test\"'test'")
    .assert_stdout("--test=2 --test=2 testTEST TESTtestTEST testtest testtesttest testtesttest\n")
    .run()
    .await;

    TestBuilder::new()
        .command("deno eval 'console.log(1)'")
        .env_var("PATH", "")
        .assert_stderr("deno: command not found\n")
        .assert_exit_code(127)
        .run()
        .await;

    TestBuilder::new().command("unset").run().await;

    TestBuilder::new()
        .command("a=1 && echo $((a=2, a + 1)) && echo $a")
        .assert_stdout("3\n2\n")
        .run()
        .await;

    TestBuilder::new()
        .command("a=1 && echo $a")
        .assert_stdout("1\n")
        .run()
        .await;
}

#[tokio::test]
async fn boolean_logic() {
    TestBuilder::new()
        .command("echo 1 && echo 2 || echo 3")
        .assert_stdout("1\n2\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo 1 || echo 2 && echo 3")
        .assert_stdout("1\n3\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo 1 || (echo 2 && echo 3)")
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command("false || false || (echo 2 && false) || echo 3")
        .assert_stdout("2\n3\n")
        .run()
        .await;
}

#[tokio::test]
async fn exit() {
    TestBuilder::new()
        .command("exit 1")
        .assert_exit_code(1)
        .run()
        .await;

    TestBuilder::new()
        .command("exit 5")
        .assert_exit_code(5)
        .run()
        .await;

    TestBuilder::new()
        .command("exit 258 && echo 1")
        .assert_exit_code(2)
        .run()
        .await;

    TestBuilder::new()
        .command("(exit 0) && echo 1")
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command("(exit 1) && echo 1")
        .assert_exit_code(1)
        .run()
        .await;

    TestBuilder::new()
        .command("echo 1 && (exit 1)")
        .assert_stdout("1\n")
        .assert_exit_code(1)
        .run()
        .await;

    TestBuilder::new()
        .command("exit ; echo 2")
        .assert_exit_code(1)
        .run()
        .await;

    TestBuilder::new()
        .command("exit bad args")
        .assert_stderr("exit: too many arguments\n")
        .assert_exit_code(2)
        .run()
        .await;
}

#[tokio::test]
async fn command_substitution() {
    TestBuilder::new()
        .command("echo $(echo 1)")
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $(echo 1 && echo 2)")
        .assert_stdout("1 2\n")
        .run()
        .await;

    // async inside subshell should wait
    TestBuilder::new()
        .command("$(sleep 0.1 && echo 1 & echo echo) 2")
        .assert_stdout("1 2\n")
        .run()
        .await;
    TestBuilder::new()
        .command("set +e\n$(sleep 0.1 && echo 1 && exit 5 &) ; echo 2")
        .assert_stdout("2\n")
        .assert_stderr("1: command not found\n")
        .run()
        .await;
}

#[tokio::test]
async fn sequential_lists() {
    TestBuilder::new()
        .command(r#"echo 1 ; sleep 0.1 && echo 4 & echo 2 ; echo 3;"#)
        .assert_stdout("1\n2\n3\n4\n")
        .run()
        .await;
}
#[tokio::test]
async fn pipeline() {
    TestBuilder::new()
        .command(r#"echo 1 | echo 2 && echo 3"#)
        .assert_stdout("2\n3\n")
        .run()
        .await;

    // TODO: implement tee in shell and then enable this test
    // TestBuilder::new()
    //     .command(r#"echo 1 | tee output.txt"#)
    //     .assert_stdout("1\n")
    //     .assert_file_equals("output.txt", "1\n")
    //     .run()
    //     .await;

    TestBuilder::new()
        .command(r#"echo 1 | cat > output.txt"#)
        .assert_file_equals("output.txt", "1\n")
        .run()
        .await;
}

#[tokio::test]
async fn redirects_input() {
    TestBuilder::new()
        .file("test.txt", "Hi!")
        .command(r#"cat - < test.txt"#)
        .assert_stdout("Hi!")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "Hi!\n")
        .command(r#"cat - < test.txt && echo There"#)
        .assert_stdout("Hi!\nThere\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"cat - <&0"#)
        .assert_stderr("shell: input redirecting file descriptors is not implemented\n")
        .assert_exit_code(1)
        .run()
        .await;
}

#[tokio::test]
async fn pwd() {
    TestBuilder::new()
        .directory("sub_dir")
        .file("file.txt", "test")
        .command("pwd && cd sub_dir && pwd && cd ../ && pwd")
        // the actual temp directory will get replaced here
        .assert_stdout(&format!(
            "$TEMP_DIR\n$TEMP_DIR{FOLDER_SEPARATOR}sub_dir\n$TEMP_DIR\n"
        ))
        .run()
        .await;

    TestBuilder::new()
        .command("pwd -M")
        .assert_stderr("pwd: unsupported flag: -M\n")
        .assert_exit_code(1)
        .run()
        .await;
}

#[tokio::test]
async fn subshells() {
    TestBuilder::new()
        .command("(export TEST=1) && echo $TEST")
        .assert_stdout("\n")
        .assert_exit_code(0)
        .run()
        .await;
    TestBuilder::new()
        .directory("sub_dir")
        .command("echo $PWD && (cd sub_dir && echo $PWD) && echo $PWD")
        .assert_stdout(&format!(
            "$TEMP_DIR\n$TEMP_DIR{FOLDER_SEPARATOR}sub_dir\n$TEMP_DIR\n"
        ))
        .assert_exit_code(0)
        .run()
        .await;
    TestBuilder::new()
        .command("export TEST=1 && (echo $TEST && unset TEST && echo $TEST) && echo $TEST")
        .assert_stdout("1\n\n1\n")
        .assert_exit_code(0)
        .run()
        .await;
    TestBuilder::new()
        .command("(exit 1) && echo 1")
        .assert_exit_code(1)
        .run()
        .await;
    TestBuilder::new()
        .command("(exit 1) || echo 1")
        .assert_stdout("1\n")
        .assert_exit_code(0)
        .run()
        .await;
}

#[tokio::test]
#[cfg(unix)]
async fn pwd_logical() {
    TestBuilder::new()
        .directory("main")
        .command("ln -s main symlinked_main && cd symlinked_main && pwd && pwd -L")
        .assert_stdout("$TEMP_DIR/symlinked_main\n$TEMP_DIR/main\n")
        .run()
        .await;
}

#[tokio::test]
async fn cat() {
    // no args
    TestBuilder::new()
        .command("cat")
        .stdin("hello")
        .assert_stdout("hello")
        .run()
        .await;

    // dash
    TestBuilder::new()
        .command("cat -")
        .stdin("hello")
        .assert_stdout("hello")
        .run()
        .await;

    // file
    TestBuilder::new()
        .command("cat file")
        .file("file", "test")
        .assert_stdout("test")
        .run()
        .await;

    // multiple files
    TestBuilder::new()
        .command("cat file1 file2")
        .file("file1", "test")
        .file("file2", "other")
        .assert_stdout("testother")
        .run()
        .await;

    // multiple files and stdin
    TestBuilder::new()
        .command("cat file1 file2 -")
        .file("file1", "test\n")
        .file("file2", "other\n")
        .stdin("hello")
        .assert_stdout("test\nother\nhello")
        .run()
        .await;

    // multiple files and stdin different order
    TestBuilder::new()
        .command("cat file1 - file2")
        .file("file1", "test\n")
        .file("file2", "other\n")
        .stdin("hello\n")
        .assert_stdout("test\nhello\nother\n")
        .run()
        .await;

    // file containing a command to evaluate
    TestBuilder::new()
        .command("$(cat file)")
        .file("file", "echo hello")
        .assert_stdout("hello\n")
        .run()
        .await;
}

#[tokio::test]
async fn head() {
    // no args
    TestBuilder::new()
        .command("head")
        .stdin("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\nplugh\n")
        .assert_stdout("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\n")
        .run()
        .await;

    // dash
    TestBuilder::new()
        .command("head -")
        .stdin("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\nplugh\n")
        .assert_stdout("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\n")
        .run()
        .await;

    // file
    TestBuilder::new()
        .command("head file")
        .file(
            "file",
            "foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\nplugh\n",
        )
        .assert_stdout("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\n")
        .run()
        .await;

    // dash + longer than internal buffer (512)
    TestBuilder::new()
        .command("head -")
        .stdin(
            "foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\nplugh\n"
                .repeat(10)
                .as_str(),
        )
        .assert_stdout("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\n")
        .run()
        .await;

    // file + longer than internal buffer (512)
    TestBuilder::new()
        .command("head file")
        .file(
            "file",
            "foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\nplugh\n"
                .repeat(1024)
                .as_str(),
        )
        .assert_stdout("foo\nbar\nbaz\nqux\nquuux\ncorge\ngrault\ngarply\nwaldo\nfred\n")
        .run()
        .await;

    // shorter than 10 lines
    TestBuilder::new()
        .command("head")
        .stdin("foo\nbar")
        .assert_stdout("foo\nbar")
        .run()
        .await;

    // -n
    TestBuilder::new()
        .command("head -n 2")
        .stdin("foo\nbar\nbaz\nqux\nquuux")
        .assert_stdout("foo\nbar\n")
        .run()
        .await;

    // --lines
    TestBuilder::new()
        .command("head --lines=3")
        .stdin("foo\nbar\nbaz\nqux\nquuux")
        .assert_stdout("foo\nbar\nbaz\n")
        .run()
        .await;
}

// Basic integration tests as there are unit tests in the commands
#[tokio::test]
async fn mv() {
    // single file
    TestBuilder::new()
        .command("mv file1.txt file2.txt")
        .file("file1.txt", "test")
        .assert_not_exists("file1.txt")
        .assert_exists("file2.txt")
        .run()
        .await;

    // multiple files to folder
    TestBuilder::new()
        .command("mkdir sub_dir && mv file1.txt file2.txt sub_dir")
        .file("file1.txt", "test1")
        .file("file2.txt", "test2")
        .assert_not_exists("file1.txt")
        .assert_not_exists("file2.txt")
        .assert_exists("sub_dir/file1.txt")
        .assert_exists("sub_dir/file2.txt")
        .run()
        .await;

    // error message
    TestBuilder::new()
        .command("mv file1.txt file2.txt")
        .assert_exit_code(1)
        .assert_stderr(&format!(
            "mv: could not move file1.txt to file2.txt: {}\n",
            no_such_file_error_text()
        ))
        .run()
        .await;
}

// Basic integration tests as there are unit tests in the commands
#[tokio::test]
async fn cp() {
    // single file
    TestBuilder::new()
        .command("cp file1.txt file2.txt")
        .file("file1.txt", "test")
        .assert_exists("file1.txt")
        .assert_exists("file2.txt")
        .run()
        .await;

    // multiple files to folder
    TestBuilder::new()
        .command("mkdir sub_dir && cp file1.txt file2.txt sub_dir")
        .file("file1.txt", "test1")
        .file("file2.txt", "test2")
        .assert_exists("file1.txt")
        .assert_exists("file2.txt")
        .assert_exists("sub_dir/file1.txt")
        .assert_exists("sub_dir/file2.txt")
        .run()
        .await;

    // error message
    TestBuilder::new()
        .command("cp file1.txt file2.txt")
        .assert_exit_code(1)
        .assert_stderr(&format!(
            "cp: could not copy file1.txt to file2.txt: {}\n",
            no_such_file_error_text()
        ))
        .run()
        .await;
}

// Basic integration tests as there are unit tests in the commands
#[tokio::test]
async fn mkdir() {
    TestBuilder::new()
        .command("mkdir sub_dir")
        .assert_exists("sub_dir")
        .run()
        .await;

    // error message
    TestBuilder::new()
        .command("mkdir file.txt")
        .file("file.txt", "test")
        .assert_stderr("mkdir: cannot create directory 'file.txt': File exists\n")
        .assert_exit_code(1)
        .run()
        .await;
}

// Basic integration tests as there are unit tests in the commands
#[tokio::test]
async fn rm() {
    TestBuilder::new()
        .command("mkdir sub_dir && rm -d sub_dir && rm file.txt")
        .file("file.txt", "")
        .assert_not_exists("sub_dir")
        .assert_not_exists("file.txt")
        .run()
        .await;

    // error message
    TestBuilder::new()
        .command("rm file.txt")
        .assert_stderr(&format!(
            "rm: cannot remove 'file.txt': {}\n",
            no_such_file_error_text()
        ))
        .assert_exit_code(1)
        .run()
        .await;
}

#[cfg(windows)]
#[tokio::test]
async fn windows_resolve_command() {
    // not cross platform, but still allow this
}

#[tokio::test]
async fn custom_command() {
    // not cross platform, but still allow this
    TestBuilder::new()
        .command("add 1 2")
        .custom_command(
            "add",
            Box::new(|mut context| {
                async move {
                    let mut sum = 0;
                    for val in context.args {
                        sum += val.parse::<usize>().unwrap();
                    }
                    let _ = context.stderr.write_line(&sum.to_string());
                    ExecuteResult::from_exit_code(0)
                }
                .boxed_local()
            }),
        )
        .assert_stderr("3\n")
        .run()
        .await;
}

#[tokio::test]
async fn glob_basic() {
    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("test2.txt", "test2\n")
        .command("cat *.txt")
        .assert_stdout("test\ntest2\n")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("test2.txt", "test2\n")
        .command("cat test?.txt")
        .assert_stdout("test2\n")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("testa.txt", "testa\n")
        .file("test2.txt", "test2\n")
        .command("cat test[0-9].txt")
        .assert_stdout("test2\n")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("testa.txt", "testa\n")
        .file("test2.txt", "test2\n")
        .command("cat test[!a-z].txt")
        .assert_stdout("test2\n")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("testa.txt", "testa\n")
        .file("test2.txt", "test2\n")
        .command("cat test[a-z].txt")
        .assert_stdout("testa\n")
        .run()
        .await;

    TestBuilder::new()
        .directory("sub_dir/sub")
        .file("sub_dir/sub/1.txt", "1\n")
        .file("sub_dir/2.txt", "2\n")
        .file("sub_dir/other.ts", "other\n")
        .file("3.txt", "3\n")
        .command("cat */*.txt")
        .assert_stdout("2\n")
        .run()
        .await;

    TestBuilder::new()
        .directory("sub_dir/sub")
        .file("sub_dir/sub/1.txt", "1\n")
        .file("sub_dir/2.txt", "2\n")
        .file("sub_dir/other.ts", "other\n")
        .file("3.txt", "3\n")
        .command("cat **/*.txt")
        .assert_stdout("3\n2\n1\n")
        .run()
        .await;

    TestBuilder::new()
        .directory("sub_dir/sub")
        .file("sub_dir/sub/1.txt", "1\n")
        .file("sub_dir/2.txt", "2\n")
        .file("sub_dir/other.ts", "other\n")
        .file("3.txt", "3\n")
        .command("cat $PWD/**/*.txt")
        .assert_stdout("3\n2\n1\n")
        .run()
        .await;

    TestBuilder::new()
        .directory("dir")
        .file("dir/1.txt", "1\n")
        .file("dir_1.txt", "2\n")
        .command("cat dir*1.txt")
        .assert_stdout("2\n")
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("test2.txt", "test2\n")
        .command("cat *.ts")
        .assert_stderr("glob: no matches found '$TEMP_DIR/*.ts'\n")
        .assert_exit_code(1)
        .run()
        .await;

    let mut builder = TestBuilder::new();
    let temp_dir_path = builder.temp_dir_path();
    let error_pos = temp_dir_path.to_string_lossy().len() + 1;
    builder.file("test.txt", "test\n")
    .file("test2.txt", "test2\n")
    .command("cat [].ts")
    .assert_stderr(&format!("glob: no matches found '$TEMP_DIR/[].ts'. Pattern syntax error near position {}: invalid range pattern\n", error_pos))
    .assert_exit_code(1)
    .run()
    .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("test2.txt", "test2\n")
        .command("cat *.ts || echo 2")
        .assert_stderr("glob: no matches found '$TEMP_DIR/*.ts'\n")
        .assert_stdout("2\n")
        .assert_exit_code(0)
        .run()
        .await;

    TestBuilder::new()
        .file("test.txt", "test\n")
        .file("test2.txt", "test2\n")
        .command("cat *.ts 2> /dev/null || echo 2")
        .assert_stderr("")
        .assert_stdout("2\n")
        .assert_exit_code(0)
        .run()
        .await;

    TestBuilder::new()
        .command("echo --inspect='[::0]:3366'")
        .assert_stderr("")
        .assert_stdout("--inspect=[::0]:3366\n")
        .assert_exit_code(0)
        .run()
        .await;
}

#[tokio::test]
async fn glob_case_insensitive() {
    TestBuilder::new()
        .file("TEST.txt", "test\n")
        .file("testa.txt", "testa\n")
        .file("test2.txt", "test2\n")
        .command("cat tes*.txt")
        .assert_stdout("test\ntest2\ntesta\n")
        .run()
        .await;
}

#[tokio::test]
async fn paren_escapes() {
    TestBuilder::new()
        .command(r"echo \( foo bar \)")
        .assert_stdout("( foo bar )\n")
        .run()
        .await;
}

#[tokio::test]
async fn uname() {
    TestBuilder::new()
        .command("uname")
        .assert_exit_code(0)
        .check_stdout(false)
        .run()
        .await;

    TestBuilder::new()
        .command("uname -a")
        .assert_exit_code(0)
        .check_stdout(false)
        .run()
        .await;
}

#[tokio::test]
async fn which() {
    TestBuilder::new()
        .command("which ls")
        .assert_exit_code(0)
        .assert_stdout("<builtin function>\n")
        .run()
        .await;

    TestBuilder::new()
        .command("which bla foo")
        .assert_exit_code(1)
        .assert_stderr("Expected one argument\n")
        .run()
        .await;

    TestBuilder::new()
        .command("alias ll=\"ls -al\" && which ll")
        .assert_exit_code(0)
        .assert_stdout("alias: \"ls -al\"\n")
        .run()
        .await;
}

#[tokio::test]
async fn arithmetic() {
    TestBuilder::new()
        .command("echo $((1 + 2 * 3 + (4 / 5)))")
        .assert_stdout("7\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((a=1, b=2))")
        .assert_stdout("2\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((a=1, b=2, a+b))")
        .assert_stdout("3\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((1 + 2))")
        .assert_stdout("3\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((5 * 4))")
        .assert_stdout("20\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((10 / 3))")
        .assert_stdout("3\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((2 ** 3))")
        .assert_stdout("8\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((2 << 3))")
        .assert_stdout("16\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo $((2 << 3))")
        .assert_stdout("16\n")
        .run()
        .await;
}

#[tokio::test]
async fn date() {
    TestBuilder::new()
        .command("date")
        .assert_exit_code(0)
        .check_stdout(false)
        .run()
        .await;

    TestBuilder::new()
        .command("date +%Y-%m-%d")
        .assert_exit_code(0)
        .check_stdout(false)
        .run()
        .await;
}

#[tokio::test]
async fn if_clause() {
    TestBuilder::new()
        .command(r#"FOO=2; if [[ $FOO == 1 ]]; then echo "FOO is 1"; elif [[ $FOO -eq 2 ]]; then echo "FOO is 2"; else echo "FOO is not 1 or 2"; fi"#)
        .assert_stdout("FOO is 2\n")
        .run()
        .await;
    TestBuilder::new()
        .command(r#"FOO=3; if [[ $FOO == 1 ]]; then echo "FOO is 1"; elif [[ $FOO -eq 2 ]]; then echo "FOO is 2"; else echo "FOO is not 1 or 2"; fi"#)
        .assert_stdout("FOO is not 1 or 2\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1; if [[ $FOO == 1 ]]; then echo "FOO is 1"; elif [[ $FOO -eq 2 ]]; then echo "FOO is 2"; else echo "FOO is not 1 or 2"; fi"#)
        .assert_stdout("FOO is 1\n")
        .run()
        .await;

    TestBuilder::new()
        .script_file("../../scripts/if_else.sh")
        .assert_exit_code(0)
        .assert_stdout("FOO is 2\n")
        .assert_stdout("FOO is 2\n")
        .assert_stdout("FOO is 2\n")
        .assert_stdout("FOO is 2\n")
        .run()
        .await;
}

#[tokio::test]
async fn touch() {
    TestBuilder::new()
        .command("touch file.txt")
        .assert_exists("file.txt")
        .check_stdout(false)
        .run()
        .await;

    TestBuilder::new()
        .command("touch -m file.txt")
        .assert_exists("file.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -c nonexistent.txt")
        .assert_not_exists("nonexistent.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch file1.txt file2.txt")
        .assert_exists("file1.txt")
        .assert_exists("file2.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -d 'Tue Feb 20 14:30:00 2024' posix_locale.txt")
        .assert_exists("posix_locale.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -d '2024-02-20' iso_8601.txt")
        .assert_exists("iso_8601.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -t 202402201430.00 yyyymmddhhmmss.txt")
        .assert_exists("yyyymmddhhmmss.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -d '2024-02-20 14:30:00.000000' yyyymmddhhmmss_ms.txt")
        .assert_exists("yyyymmddhhmmss_ms.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -d '2024-02-20 14:30' yyyy_mm_dd_hh_mm.txt")
        .assert_exists("yyyy_mm_dd_hh_mm.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -t 202402201430 yyyymmddhhmm.txt")
        .assert_exists("yyyymmddhhmm.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch -d '2024-02-20 14:30 +0000' yyyymmddhhmm_offset.txt")
        .assert_exists("yyyymmddhhmm_offset.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch file.txt && touch -r file.txt reference.txt")
        .assert_exists("reference.txt")
        .run()
        .await;
    // Test for non-existent file with -c option
    TestBuilder::new()
        .command("touch -c nonexistent.txt")
        .assert_not_exists("nonexistent.txt")
        .run()
        .await;

    // Test for invalid date format
    TestBuilder::new()
        .command("touch -d 'invalid date' invalid_date.txt")
        .assert_stderr_contains("Unable to parse date: invalid date\n")
        .assert_exit_code(1)
        .run()
        .await;

    // Test for invalid timestamp format
    TestBuilder::new()
        .command("touch -t 9999999999 invalid_timestamp.txt")
        .assert_stderr_contains("invalid date format '9999999999'\n")
        .assert_exit_code(1)
        .run()
        .await;

    TestBuilder::new()
        .command("touch $TEMP_DIR/absolute_path.txt")
        .assert_exists("$TEMP_DIR/absolute_path.txt")
        .run()
        .await;

    TestBuilder::new()
        .command("touch $TEMP_DIR/non_existent_dir/non_existent.txt")
        .assert_stderr_contains("No such file or directory")
        .assert_exit_code(1)
        .run()
        .await;

    // TODO: implement ln in shell and then enable this test
    // // Test with -h option on a symlink
    // TestBuilder::new()
    //     .command("touch original.txt && ln -s original.txt symlink.txt && touch -h symlink.txt")
    //     .assert_exists("symlink.txt")
    //     .run()
    //     .await;

    // Test with multiple files, including one that doesn't exist
    TestBuilder::new()
        .command("touch existing.txt && touch existing.txt nonexistent.txt another_existing.txt")
        .assert_exists("existing.txt")
        .assert_exists("nonexistent.txt")
        .assert_exists("another_existing.txt")
        .run()
        .await;
}

#[tokio::test]
async fn variable_expansion() {
    // DEFAULT VALUE EXPANSION
    TestBuilder::new()
        .command("echo ${FOO:-5}")
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${FOO:-5}""#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo ${FOO:-5}"#)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo "${FOO:-5}""#)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo ${FOO:-${BAR:-5}}"#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${FOO:-${BAR:-5}}""#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command("BAR=2 && echo ${FOO:-${BAR:-5}}")
        .assert_stdout("2\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"BAR=2 && echo "${FOO:-${BAR:-5}}""#)
        .assert_stdout("2\n")
        .run()
        .await;

    TestBuilder::new()
        .command("echo ${BAR:-THE VALUE CAN CONTAIN SPACES}")
        .assert_stdout("THE VALUE CAN CONTAIN SPACES\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${BAR:-THE VALUE CAN CONTAIN SPACES}""#)
        .assert_stdout("THE VALUE CAN CONTAIN SPACES\n")
        .run()
        .await;

    // ASSIGN DEFAULT EXPANSION
    TestBuilder::new()
        .command("echo ${FOO:=5} && echo $FOO")
        .assert_stdout("5\n5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${FOO:=5}" && echo "$FOO""#)
        .assert_stdout("5\n5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo ${FOO:=5} && echo $FOO"#)
        .assert_stdout("1\n1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo "${FOO:=5}" && echo "$FOO""#)
        .assert_stdout("1\n1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo ${FOO:=${BAR:=5}} && echo $FOO && echo $BAR"#)
        .assert_stdout("5\n5\n5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${FOO:=${BAR:=5}}" && echo "$FOO" && echo "$BAR""#)
        .assert_stdout("5\n5\n5\n")
        .run()
        .await;

    // SUBSTRING VARIABLE EXPANSION
    TestBuilder::new()
        .command(r#"FOO=12345 && echo ${FOO:1:3}"#)
        .assert_stdout("234\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO:1:3}""#)
        .assert_stdout("234\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo ${FOO:1}"#)
        .assert_stdout("2345\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO:1}""#)
        .assert_stdout("2345\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo ${FOO:1:-1}"#)
        .assert_stdout("234\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO:1:-1}""#)
        .assert_stdout("234\n")
        .run()
        .await;

    // ALTERNATE VALUE EXPANSION
    TestBuilder::new()
        .command(r#"FOO=1 && echo ${FOO:+5}"#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo "${FOO:+5}""#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo ${FOO:+5}"#)
        .assert_stdout("\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"echo "${FOO:+5}""#)
        .assert_stdout("\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo ${FOO:+${BAR:+5}}"#)
        .assert_stdout("\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && echo "${FOO:+${BAR:+5}}""#)
        .assert_stdout("\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && BAR=2 && echo ${FOO:+${BAR:+5}}"#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=1 && BAR=2 && echo "${FOO:+${BAR:+5}}""#)
        .assert_stdout("5\n")
        .run()
        .await;

    TestBuilder::new()
        .command("FOO=12345 && echo ${FOO:2:$((2+2))}")
        .assert_stdout("345\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO:2:$((2+2))}""#)
        .assert_stdout("345\n")
        .run()
        .await;

    TestBuilder::new()
        .command("FOO=12345 && echo ${FOO: -2:-1}")
        .assert_stdout("4\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO: -2:-1}""#)
        .assert_stdout("4\n")
        .run()
        .await;

    TestBuilder::new()
        .command("FOO=12345 && echo ${FOO: -2}")
        .assert_stdout("45\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO: -2}""#)
        .assert_stdout("45\n")
        .run()
        .await;

    TestBuilder::new()
        .command("FOO=12345 && echo ${FOO: -4: 2}")
        .assert_stdout("23\n")
        .run()
        .await;

    TestBuilder::new()
        .command(r#"FOO=12345 && echo "${FOO: -4: 2}""#)
        .assert_stdout("23\n")
        .run()
        .await;
}

#[tokio::test]
async fn test_set() {
    let no_such_file_error_text = no_such_file_error_text();

    TestBuilder::new()
        .command(
            r#"
        set -e
        FOO=1 && echo $FOO
        cat no_existent.txt
        echo "This should not be printed"
        "#,
        )
        .assert_exit_code(1)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(
            r#"
        set +e
        FOO=1 && echo $FOO
        cat no_existent.txt
        echo "This should be printed"
        "#,
        )
        .assert_exit_code(0)
        .assert_stdout("1\nThis should be printed\n")
        .run()
        .await;

    TestBuilder::new() // set -e should be set by default
        .command(
            r#"
        FOO=1 && echo $FOO
        cat no_existent.txt
        echo "This should not be printed"
        "#,
        )
        .assert_exit_code(1)
        .assert_stdout("1\n")
        .run()
        .await;

    TestBuilder::new() // set -e behavior in pipelines
        .command(
            r#"
        set -e
        echo "hi" && cat nonexistent.txt || echo "This should be printed"
        echo "This should also be printed"
        "#,
        )
        .assert_exit_code(0)
        .assert_stdout("hi\nThis should be printed\nThis should also be printed\n")
        .assert_stderr(&format!(
            "cat: nonexistent.txt: {no_such_file_error_text}\n"
        ))
        .run()
        .await;

    TestBuilder::new() // set -e behavior with subshells
        .command(
            r#"
        set -e
        (echo "hi" && cat nonexistent.txt) || echo "This should be printed"
        echo "This should also be printed"
        "#,
        )
        .assert_exit_code(0)
        .assert_stdout("hi\nThis should be printed\nThis should also be printed\n")
        .assert_stderr(&format!(
            "cat: nonexistent.txt: {no_such_file_error_text}\n"
        ))
        .run()
        .await;

    TestBuilder::new() // updating shell's state in a command
        .command(
            r#"
        set +e
        cat no_existent.txt
        echo "This should be printed"
        set -e
        cat no_existent.txt
        echo "This should not be printed"
        "#,
        )
        .assert_exit_code(1)
        .assert_stdout("This should be printed\n")
        .assert_stderr(&format!("cat: no_existent.txt: {no_such_file_error_text}\ncat: no_existent.txt: {no_such_file_error_text}\n"))
        .run()
        .await;

    // Tests for set -x
    TestBuilder::new()
        .command(
            r#"
        set -x
        echo "hi"
        "#,
        )
        .assert_stdout("+ echo hi\nhi\n")
        .run()
        .await;

    TestBuilder::new()
        .command(
            r#"
        set -x
        echo "hi" && echo "This should be printed" || echo "This should not be printed"
        "#,
        )
        .assert_stdout("+ echo hi\nhi\n+ echo This should be printed\nThis should be printed\n")
        .run()
        .await;

    TestBuilder::new()
        .command(
            r#"
        set -x
        FOO=1
        echo $FOO
        "#,
        )
        .assert_stdout("+ FOO=1\n+ echo 1\n1\n")
        .run()
        .await;

    TestBuilder::new()
        .command(
            r#"
        set -x
        FOO=1
        echo $FOO
        set +x
        echo "This should be printed"
        "#,
        )
        .assert_stdout("+ FOO=1\n+ echo 1\n1\n+ set +x\nThis should be printed\n")
        .run()
        .await;

    TestBuilder::new()
        .command(
            r#"set -x
            echo $((10 + 20))
        "#,
        )
        .assert_stdout("+ echo 30\n30\n")
        .run()
        .await;
}

#[tokio::test]
async fn test_reserved_substring() {
    // Test that there is no panic (prefix-dev/shell#256)
    TestBuilder::new()
        .command(r#"find . -name 'platform*'"#)
        .assert_exit_code(1)
        .run()
        .await;
}

#[cfg(test)]
fn no_such_file_error_text() -> &'static str {
    if cfg!(windows) {
        "The system cannot find the file specified. (os error 2)"
    } else {
        "No such file or directory (os error 2)"
    }
}
