use anyhow::Context;
use deno_task_shell::{
    execute_sequential_list, AsyncCommandBehavior, ExecuteResult, ShellPipeReader, ShellPipeWriter,
    ShellState,
};

pub async fn execute(text: &str, state: &mut ShellState) -> anyhow::Result<i32> {
    let list = deno_task_shell::parser::parse(text);

    let mut stderr = ShellPipeWriter::stderr();
    let stdout = ShellPipeWriter::stdout();
    let stdin = ShellPipeReader::stdin();

    if let Err(e) = list {
        let _ = stderr.write_line(&format!("Syntax error: {}", e));
        return Ok(1);
    }

    // spawn a sequential list and pipe its output to the environment
    let result = execute_sequential_list(
        list.unwrap(),
        state.clone(),
        stdin,
        stdout,
        stderr,
        AsyncCommandBehavior::Wait,
    )
    .await;

    match result {
        ExecuteResult::Continue(exit_code, changes, _) => {
            // set CWD to the last command's CWD
            state.apply_changes(&changes);
            std::env::set_current_dir(state.cwd()).context("Failed to set CWD")?;
            Ok(exit_code)
        }
        ExecuteResult::Exit(_, _) => Ok(0),
    }
}
