use deno_task_shell::{
    execute_sequential_list, AsyncCommandBehavior, ExecuteResult, ShellPipeReader, ShellPipeWriter,
    ShellState,
};
use miette::{Context, IntoDiagnostic};

pub async fn execute_inner(
    text: &str,
    filename: Option<String>,
    state: ShellState,
) -> miette::Result<ExecuteResult> {
    let list = deno_task_shell::parser::parse(text);

    let mut stderr = ShellPipeWriter::stderr();
    let stdout = ShellPipeWriter::stdout();
    let stdin = ShellPipeReader::stdin();

    if let Err(e) = list {
        if let Some(filename) = &filename {
            stderr.write_all(format!("Filename: {:?}\n", filename).as_bytes())?;
        }
        stderr.write_all(format!("Syntax error: {:?}", e).as_bytes())?;
        return Ok(ExecuteResult::Exit(1, vec![], vec![]));
    }

    // spawn a sequential list and pipe its output to the environment
    let result = execute_sequential_list(
        list.unwrap(),
        state,
        stdin,
        stdout,
        stderr,
        AsyncCommandBehavior::Wait,
    )
    .await;

    Ok(result)
}

pub async fn execute(
    text: &str,
    filename: Option<String>,
    state: &mut ShellState,
) -> miette::Result<ExecuteResult> {
    let result = execute_inner(text, filename, state.clone()).await?;

    let changes = match &result {
        ExecuteResult::Exit(_, changes, _) => changes,
        ExecuteResult::Continue(_, changes, _) => changes,
    };
    // set CWD to the last command's CWD
    state.apply_changes(changes);
    std::env::set_current_dir(state.cwd())
        .into_diagnostic()
        .context("Failed to set CWD")?;

    Ok(result)
}
