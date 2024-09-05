use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

async fn execute(text: &str) -> anyhow::Result<i32> {
    let list = deno_task_shell::parser::parse(text)?;

    // execute
    let env_vars = std::env::vars().collect();

    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    let exit_code = deno_task_shell::execute(
        list,
        env_vars,
        &cwd,
        Default::default(), // custom commands
    )
    .await;

    Ok(exit_code)
}

#[derive(Parser)]
struct Options {
    #[clap(short, long)]
    file: Option<PathBuf>,
}

async fn interactive() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;

    let mut prev_exit_code = 0;
    loop {
        // Display the prompt and read a line
        let readline = if prev_exit_code == 0 {
            rl.readline(">>> ")
        } else {
            rl.readline("xxx ")
        };

        match readline {
            Ok(line) => {
                // Add the line to history
                rl.add_history_entry(line.as_str())?;

                // Process the input (here we just echo it back)
                prev_exit_code = execute(&line).await.context("Failed to execute")?;

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
        execute(&script_text).await?;
    } else {
        interactive().await?;
    }

    Ok(())
}
