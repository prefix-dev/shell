![image](https://github.com/user-attachments/assets/74ad3cdd-9890-4b41-b42f-7eaed269f505)

# 🦀 shell - fast, cross-platform Bash compatible shell 🚀

This shell looks and feels like bash, but works **natively on Windows** (and macOS / Linux)! No emulation needed.

The idea of the `shell` project is to build a cross-platform shell that looks and feels similar to bash (while not claiming to be 100% bash compatible). The `shell` allows you to use platform specific native operations (e.g. `cd 'C:\Program Files (x86)'` on Windows), but it also allows you to use a platform-independent strict subset of bash which enables writing build scripts and instructions that work on all platforms.

The project is written in Rust.

The most common bash commands are implemented and we are linking with the `coreutils` crate to provide the most important Unix commands in a cross-platform, memory safe way (such as `mv`, `cp`, `ls`, `cat`, etc.).

This new shell also already has _tab completion_ for files and directories, and _history_ support thanks to `rustyline`.

The project is still very early alpha stage but can already be used as a daily
driver on all platforms.

## Screenshots

macOS:

[](https://github.com/user-attachments/assets/7f5c72ed-2bce-4f64-8a53-792d153cf574)

Windows:

![Windows](https://github.com/user-attachments/assets/6982534c-066e-4b26-a1ec-b11cea7a3ffb)

## How to run this

To compile and run the project, you need to have Rust & Cargo installed.

```bash
# To start an interactive shell
cargo r

# To run a script
cargo r -- ./scripts/hello_world.sh

# To run a script and continue in interactive mode
cargo r -- ./scripts/hello_world.sh --interact
```

## License

The project is licensed under the MIT License. It is an extension of the existing `deno_task_shell` project (also licensed under the MIT License, by the authors of `deno`).
