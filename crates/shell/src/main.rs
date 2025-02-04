use std::collections::HashMap;
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

    /// Do not source ~/.shellrc on startup
    #[clap(long)]
    norc: bool,

    /// Execute a command
    #[clap(short)]
    command: Option<String>,

    #[clap(short, long)]
    debug: bool,

    // Trailing args to forward to the script
    #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
    args: Vec<String>,
}

async fn init_state(norc: bool, var_args: &[String]) -> miette::Result<ShellState> {
    let mut env_vars: HashMap<String, String> = std::env::vars().collect();
    let default_ps1 = "{display_cwd}{git_branch}$ ";
    env_vars.insert("PS1".to_string(), default_ps1.to_string());

    let mut shell_vars = HashMap::new();
    // Set all arguments such as $0, $1, $2, etc.
    for (idx, arg) in var_args.iter().enumerate() {
        shell_vars.insert(format!("{}", idx + 1), arg.clone());
    }

    // Set the $@ variable
    let args: Vec<String> = std::env::args().collect();
    shell_vars.insert("@".to_string(), args.join(" "));
    shell_vars.insert("#".to_string(), args.len().to_string());

    // Set the SHELL variable
    let current_exe = std::env::current_exe().into_diagnostic()?;
    env_vars.insert(
        "SHELL".to_string(),
        current_exe.to_string_lossy().to_string(),
    );

    let cwd = std::env::current_dir().unwrap();
    let mut state =
        ShellState::new(env_vars, &cwd, commands::get_commands()).with_shell_vars(shell_vars);

    // Load ~/.shellrc
    if let Some(home_dir) = dirs::home_dir() {
        let shellrc_file = home_dir.join(".shellrc");
        if !norc && shellrc_file.exists() {
            let line = format!("source '{}'", shellrc_file.to_string_lossy());
            let prev_exit_code = execute(
                &line,
                Some(shellrc_file.as_path().display().to_string()),
                &mut state,
            )
            .await
            .context("Failed to source ~/.shellrc")?;
            state.set_last_command_exit_code(prev_exit_code);
        }
    }

    Ok(state)
}

async fn interactive(state: Option<ShellState>, norc: bool, args: &[String]) -> miette::Result<()> {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();

    ctrlc::set_handler(move || {
        println!("Received Ctrl+C");
    })
    .expect("Error setting Ctrl-C handler");

    let mut rl = Editor::with_config(config).into_diagnostic()?;
    let builtins = deno_task_shell::builtin_commands()
        .keys()
        .chain(commands::get_commands().keys())
        .map(|s| s.to_string())
        .collect();

    let helper = helper::ShellPromptHelper::new(builtins);
    rl.set_helper(Some(helper));

    let mut state = match state {
        Some(state) => state,
        None => init_state(norc, args).await?,
    };

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

            let mut display_cwd = if let Some(stripped) = cwd.strip_prefix(home_str) {
                format!("~{}", stripped.replace('\\', "/"))
            } else {
                cwd.to_string()
            };

            // Read the PS1 environment variable
            let ps1 = state.env_vars().get("PS1").map_or("", |v| v);

            fn replace_placeholders(ps1: &str, display_cwd: &str, git_branch: &str) -> String {
                ps1.replace(&format!("{{{}}}", "display_cwd"), display_cwd)
                    .replace(&format!("{{{}}}", "git_branch"), git_branch)
            }

            let prompt = replace_placeholders(ps1, &display_cwd, &git_branch);
            display_cwd = format!("\x1b[34m{display_cwd}\x1b[0m");
            git_branch = format!("\x1b[32m{git_branch}\x1b[0m");
            let color_prompt = replace_placeholders(ps1, &display_cwd, &git_branch);
            rl.helper_mut().unwrap().colored_prompt = color_prompt;
            rl.readline(&prompt)
        };

        match readline {
            Ok(line) => {
                // Add the line to history
                rl.add_history_entry(line.as_str()).into_diagnostic()?;

                // Process the input (here we just echo it back)
                let prev_exit_code = execute(&line, None, &mut state)
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
    let mut state = init_state(options.norc, &options.args).await?;

    match (options.file, options.command) {
        (None, None) => {
            // Interactive mode only
            interactive(None, options.norc, &options.args).await
        }
        (file, command) => {
            // Handle script file or command
            let (script_text, filename) = get_script_content(file, command)?;

            if options.debug {
                debug_parse(&script_text);
                return Ok(());
            }

            let exit_code = execute(&script_text, filename, &mut state).await?;

            if options.interact {
                interactive(Some(state), options.norc, &options.args).await?;
            }

            std::process::exit(exit_code);
        }
    }
}

fn get_script_content(
    file: Option<PathBuf>,
    command: Option<String>,
) -> miette::Result<(String, Option<String>)> {
    match (file, command) {
        (Some(path), _) => {
            let content = std::fs::read_to_string(&path)
                .into_diagnostic()
                .context("Failed to read script file")?;
            Ok((content, Some(path.display().to_string())))
        }
        (_, Some(cmd)) => Ok((cmd, None)),
        (None, None) => unreachable!(),
    }
}
