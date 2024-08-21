/// Entry point of the `shell` cli.
#[tokio::main]
async fn main() {
    // read text of argv[1] and print it
    let args: Vec<String> = std::env::args().collect();
    println!("args: {:?}", args);
    if args.len() < 2 {
        println!("Usage: {} <script>", args[0]);
        std::process::exit(1);
    }

    // read text from stdin and print it
    let script_text = std::fs::read_to_string(&args[1]).unwrap();
    println!(
        "Executing:\n\n{}\n\n-----------------------------------\n\n",
        script_text
    );

    let list = deno_task_shell::parser::parse(&script_text).unwrap();

    // // execute
    let env_vars = std::env::vars().collect();

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let exit_code = deno_task_shell::execute(
        list,
        env_vars,
        &cwd,
        Default::default(), // custom commands
    )
    .await;

    std::process::exit(exit_code);
}
