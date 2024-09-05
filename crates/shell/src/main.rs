use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

async fn execute(text: &str) {
    let list = deno_task_shell::parser::parse(text).unwrap();

    // execute
    let env_vars = std::env::vars().collect();

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let exit_code = deno_task_shell::execute(
        list,
        env_vars,
        &cwd,
        Default::default(), // custom commands
    )
    .await;

}

#[tokio::main]
async fn main() -> rustyline::Result<()> {
    // Create a new rustyline editor
    let mut rl = DefaultEditor::new()?;

    loop {
        // Display the prompt and read a line
        let readline = rl.readline(">>> ");

        match readline {
            Ok(line) => {
                // Add the line to history
                rl.add_history_entry(line.as_str())?;

                // Process the input (here we just echo it back)
                execute(&line).await;

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