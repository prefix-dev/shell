use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Context;
use clap::Parser;
use deno_task_shell::parser::debug_parse;
use deno_task_shell::{
    execute_sequential_list, AsyncCommandBehavior, ExecuteResult, ShellCommand, ShellPipeReader,
    ShellPipeWriter, ShellState,
};
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

mod commands;
mod completion;
mod helper;

fn commands() -> HashMap<String, Rc<dyn ShellCommand>> {
    HashMap::from([(
        "ls".to_string(),
        Rc::new(commands::LsCommand) as Rc<dyn ShellCommand>,
    )])
}

async fn execute(text: &str, state: &mut ShellState) -> anyhow::Result<i32> {
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

    #[clap(short, long)]
    debug: bool,
}

fn init_state() -> ShellState {
    let env_vars = std::env::vars().collect();
    let cwd = std::env::current_dir().unwrap();
    ShellState::new(env_vars, &cwd, commands())
}

async fn interactive() -> anyhow::Result<()> {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::Circular)
        .build();

    let mut rl = Editor::with_config(config)?;

    let helper = helper::ShellPromptHelper::default();
    rl.set_helper(Some(helper));

    // let h = ShellCompleter {};

    // rl.set_helper(Some(h));

    let mut state = init_state();

    let home = dirs::home_dir().context("Couldn't get home directory")?;

    let mut _prev_exit_code = 0;
    loop {
        // Reset cancellation flag
        state.reset_cancellation_token();

        // Display the prompt and read a line
        let readline = {
            let cwd = state.cwd().to_string_lossy().to_string();
            let home_str = home
                .to_str()
                .context("Couldn't convert home directory path to UTF-8 string")?;
            if !state.last_command_cd() {
                state.update_git_branch();
            }

            let mut git_branch: String = "".to_string();
            if state.git_repository() {
                git_branch = match state.git_branch().strip_prefix("ref: refs/heads/") {
                    Some(stripped) => stripped.to_string(),
                    None => {
                        let mut hash = state.git_branch().to_string();
                        if hash.len() > 7 {
                            hash = hash[0..7].to_string() + "...";
                        }
                        hash
                    }
                };
                git_branch = "(".to_owned() + &git_branch + ")";
            }

            let display_cwd = if let Some(stripped) = cwd.strip_prefix(home_str) {
                format!("~{}", stripped.replace('\\', "/"))
            } else {
                cwd.to_string()
            };

            let prompt = format!("{}{git_branch}$ ", display_cwd);
            let color_prompt = format!("\x1b[34m{}\x1b[31m{git_branch}\x1b[0m$ ", display_cwd);
            rl.helper_mut().unwrap().colored_prompt = color_prompt;
            rl.readline(&prompt)
        };

        match readline {
            Ok(line) => {
                // Add the line to history
                rl.add_history_entry(line.as_str())?;

                // Process the input (here we just echo it back)
                _prev_exit_code = execute(&line, &mut state)
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
        if options.debug {
            debug_parse(&script_text);
            return Ok(());
        }
        execute(&script_text, &mut state).await?;
    } else {
        interactive().await?;
    }

    Ok(())
}
