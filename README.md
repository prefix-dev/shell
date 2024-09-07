# shell - cross-platform bash compatible shell

The idea of the `shell` project is to build a cross-platform shell that looks and feels similar to bash (while not claiming to be 100% bash compatible).
The project is written in Rust.

The most common bash commands are implemented and we are linking with the `coreutils` crate to provide the most important Unix commands in a cross-platform, memory safe way (such as `mv`, `cp`, `ls`, `cat`, etc.).

The project is still very early but can already be used as a daily driver.

## How to run this

To compile and run the project, you need to have Rust & Cargo installed.

```bash
# To start an interactive shell
cargo r

# To run a script
cargo r -- -f ./scripts/hello_world.sh
```
