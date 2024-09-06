use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Context;
use clap::Parser;
use deno_task_shell::{
    execute_sequential_list, AsyncCommandBehavior, ExecuteResult, ShellCommand, ShellPipeReader,
    ShellPipeWriter, ShellState,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

mod commands;

fn commands() -> HashMap<String, Rc<dyn ShellCommand>> {
    HashMap::from([(
        "ls".to_string(),
        Rc::new(commands::LsCommand) as Rc<dyn ShellCommand>,
    )])
}

async fn execute(text: &str, state: &mut ShellState) -> anyhow::Result<i32> {
    let list = deno_task_shell::parser::parse(text)?;

    // spawn a sequential list and pipe its output to the environment
    let result = execute_sequential_list(
        list,
        state.clone(),
        ShellPipeReader::stdin(),
        ShellPipeWriter::stdout(),
        ShellPipeWriter::stderr(),
        AsyncCommandBehavior::Wait,
    )
    .await;

    match result {
        ExecuteResult::Continue(exit_code, changes, _) => {
            state.apply_changes(&changes);
            // set CWD to the last command's CWD
            std::env::set_current_dir(state.cwd()).context("Failed to set CWD")?;
            Ok(exit_code)
        }
        ExecuteResult::Exit(_, _) => Ok(0),
    }
}

#[derive(Parser)]
struct Options {
    #[clap(short, long)]
    file: Option<PathBuf>,
}

fn init_state() -> ShellState {
    let env_vars = std::env::vars().collect();
    let cwd = std::env::current_dir().unwrap();
    ShellState::new(env_vars, &cwd, commands())
}

async fn interactive() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;

    let mut state = init_state();

    let mut prev_exit_code = 0;
    loop {
        // Display the prompt and read a line
        let readline = if prev_exit_code == 0 {
            rl.readline(&format!("{:?} >>> ", state.cwd().to_string_lossy()))
        } else {
            rl.readline("xxx ")
        };

        match readline {
            Ok(line) => {
                // Add the line to history
                rl.add_history_entry(line.as_str())?;

                // Process the input (here we just echo it back)
                prev_exit_code = execute(&line, &mut state)
                    .await
                    .context("Failed to execute")?;

                // Check for exit command
                if line.trim().eq_ignore_ascii_case("exit") {
                    println!("Exiting...");
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    if let Some(file) = options.file {
        let script_text = std::fs::read_to_string(&file).unwrap();
        let mut state = init_state();
        execute(&script_text, &mut state).await?;
    } else {
        interactive().await?;
    }

    Ok(())
}
