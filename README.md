![image](https://github.com/user-attachments/assets/9d56a2de-b41a-49a0-b724-f5a948e449fc)

# shell - cross-platform bash compatible shell

This shell looks and feels like bash, but works **natively on Windows** (and macOS / Linux)! No emulation needed.

The idea of the `shell` project is to build a cross-platform shell that looks and feels similar to bash (while not claiming to be 100% bash compatible).
The project is written in Rust.

The most common bash commands are implemented and we are linking with the `coreutils` crate to provide the most important Unix commands in a cross-platform, memory safe way (such as `mv`, `cp`, `ls`, `cat`, etc.).

This new shell also already has _tab completion_ for files and directories, and _history_ support thanks to `rustyline`.

The project is still very early but can already be used as a daily driver.

## Screenshots

macOS:

[](https://github.com/user-attachments/assets/7f5c72ed-2bce-4f64-8a53-792d153cf574)

Windows:

![Windows](https://github.com/user-attachments/assets/f6f40994-f28d-483e-9a79-adcefeb9ae8e)

## How to run this

To compile and run the project, you need to have Rust & Cargo installed.

```bash
# To start an interactive shell
cargo r

# To run a script
cargo r -- -f ./scripts/hello_world.sh
```
