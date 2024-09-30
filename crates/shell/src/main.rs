use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use deno_task_shell::parser::debug_parse;
use deno_task_shell::ShellState;
use miette::Context;
use miette::IntoDiagnostic;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

mod commands;
mod completion;
mod execute;
mod helper;

pub use execute::execute;
#[derive(Parser)]
struct Options {
    /// The path to the file that should be executed
    file: Option<PathBuf>,

    /// Continue in interactive mode after the file has been executed
    #[clap(long)]
    interact: bool,

    #[clap(short, long)]
    debug: bool,
}

fn init_state() -> ShellState {
    let env_vars = std::env::vars().collect();
    let cwd = std::env::current_dir().unwrap();
    ShellState::new(env_vars, &cwd, commands::get_commands())
}

async fn interactive(state: Option<ShellState>) -> miette::Result<()> {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();

    ctrlc::set_handler(move || {
        println!("Received Ctrl+C");
    })
    .expect("Error setting Ctrl-C handler");

    let mut rl = Editor::with_config(config).into_diagnostic()?;

    let helper = helper::ShellPromptHelper::default();
    rl.set_helper(Some(helper));

    let mut state = state.unwrap_or_else(init_state);

    let home = dirs::home_dir().ok_or(miette::miette!("Couldn't get home directory"))?;

    // Load .shell_history
    let history_file: PathBuf = [home.as_path(), Path::new(".shell_history")]
        .iter()
        .collect();
    if Path::new(history_file.as_path()).exists() {
        rl.load_history(history_file.as_path())
            .into_diagnostic()
            .context("Failed to read the command history")?;
    }

    // Load ~/.shellrc
    let shellrc_file: PathBuf = [home.as_path(), Path::new(".shellrc")].iter().collect();
    if Path::new(shellrc_file.as_path()).exists() {
        let line = "source '".to_owned() + shellrc_file.to_str().unwrap() + "'";
        let prev_exit_code = execute(&line, &mut state)
            .await
            .context("Failed to source ~/.shellrc")?;
        state.set_last_command_exit_code(prev_exit_code);
    }

    let mut _prev_exit_code = 0;
    loop {
        // Reset cancellation flag
        state.reset_cancellation_token();

        // Display the prompt and read a line
        let readline = {
            let cwd = state.cwd().to_string_lossy().to_string();
            let home_str = home.to_str().ok_or(miette::miette!(
                "Couldn't convert home directory path to UTF-8 string"
            ))?;
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
                rl.add_history_entry(line.as_str()).into_diagnostic()?;

                // Process the input (here we just echo it back)
                let prev_exit_code = execute(&line, &mut state)
                    .await
                    .context("Failed to execute")?;
                state.set_last_command_exit_code(prev_exit_code);

                // Check for exit command
                if line.trim().eq_ignore_ascii_case("exit") {
                    println!("Exiting...");
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                // We start a new prompt on Ctrl-C, like Bash does
                println!("CTRL-C");
            }
            Err(ReadlineError::Eof) => {
                // We exit the shell on Ctrl-D, like Bash does
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history(history_file.as_path())
        .into_diagnostic()
        .context("Failed to write the command history")?;

    Ok(())
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let options = Options::parse();

    if let Some(file) = options.file {
        let script_text = std::fs::read_to_string(&file).unwrap();
        let mut state = init_state();
        if options.debug {
            debug_parse(&script_text);
            return Ok(());
        }
        execute(&script_text, &mut state).await?;
        if options.interact {
            interactive(Some(state)).await?;
        }
    } else {
        interactive(None).await?;
    }

    Ok(())
}
