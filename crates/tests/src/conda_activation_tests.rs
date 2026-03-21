/// Tests for conda-forge activation scripts.
///
/// These tests verify that our shell can parse and execute real activation
/// scripts from conda-forge feedstocks. The scripts are stored locally
/// after being downloaded by `scripts/test_conda_activation.sh`.
///
/// The scripts are preprocessed to replace conda-build template variables
/// (@VAR@) with plausible dummy values before execution.
use crate::test_builder::TestBuilder;

/// Replace conda-build template variables (@VAR@) with plausible dummy values.
fn preprocess_conda_template(script: &str) -> String {
    let replacements = [
        ("@CHOST@", "x86_64-conda-linux-gnu"),
        ("@CONDA_TOOLCHAIN_HOST@", "x86_64-conda-linux-gnu"),
        ("@CONDA_TOOLCHAIN_BUILD@", "x86_64-conda-linux-gnu"),
        ("@MACH@", "x86_64"),
        ("@VENDOR@", "conda"),
        ("@OS@", "linux"),
        ("@cross_target_platform@", "linux-64"),
        ("@native_compiler_subdir@", "linux-64"),
        ("@target_platform@", "linux-64"),
        ("@build_platform@", "linux-64"),
        ("@c_compiler@", "gcc"),
        ("@cxx_compiler@", "g++"),
        ("@fortran_compiler@", "gfortran"),
        ("@c_compiler_version@", "12"),
        ("@cxx_compiler_version@", "12"),
        ("@fortran_compiler_version@", "12"),
        ("@CC@", "gcc"),
        ("@CXX@", "g++"),
        ("@FC@", "gfortran"),
        ("@IS_WIN@", "False"),
        ("@CMAKE_SYSTEM_NAME@", "Linux"),
        ("@CONDA_BUILD_CROSS_COMPILATION@", "0"),
        (
            "@CONDA_BUILD_SYSROOT@",
            "/opt/conda/x86_64-conda-linux-gnu/sysroot",
        ),
        ("@rust_arch@", "x86_64-unknown-linux-gnu"),
        ("@rust_arch_env@", "X86_64_UNKNOWN_LINUX_GNU"),
        ("@rust_arch_env_build@", "X86_64_UNKNOWN_LINUX_GNU"),
        ("@rust_default_cc@", "x86_64-conda-linux-gnu-cc"),
        ("@rust_default_cc_build@", "x86_64-conda-linux-gnu-cc"),
        ("@CONDA_RUST_HOST_LOWER@", "x86_64_unknown_linux_gnu"),
        ("@CONDA_RUST_TARGET_LOWER@", "x86_64_unknown_linux_gnu"),
        ("@GOARCH@", "amd64"),
        ("@GOOS@", "linux"),
        ("@CGO_ENABLED@", "1"),
        ("@MACOSX_SDK_VERSION@", "10.15"),
    ];

    let mut result = script.to_string();
    for (pattern, replacement) in &replacements {
        result = result.replace(pattern, replacement);
    }

    // Replace any remaining @VAR@ patterns with a dummy value
    let re = regex::Regex::new(r"@[A-Za-z_][A-Za-z_0-9]*@").unwrap();
    result = re.replace_all(&result, "TEMPLATE_VALUE").to_string();

    result
}

async fn run_conda_script(script: &str) {
    let preprocessed = preprocess_conda_template(script);
    TestBuilder::new()
        .command(&preprocessed)
        .check_stdout(false)
        .env_var("CONDA_PREFIX", "/tmp/test_conda_prefix")
        .env_var("PREFIX", "/tmp/test_conda_prefix")
        .env_var("BUILD_PREFIX", "/tmp/test_conda_build_prefix")
        .env_var("CONDA_BUILD", "")
        .run()
        .await;
}

// === Simple activation scripts (export/unset only) ===

#[tokio::test]
async fn conda_go_activate() {
    // go-activation-feedstock: recipe/activate.sh
    // Simple exports of env vars
    run_conda_script(
        r#"export CGO_ENABLED=${CGO_ENABLED}
export GOOS=${GOOS}
export GOARCH=${GOARCH}
export CONDA_GO_COMPILER=1
export GOFLAGS="-modcacherw -buildmode=pie -trimpath"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_go_deactivate() {
    // go-activation-feedstock: recipe/deactivate.sh
    run_conda_script("unset CONDA_GO_COMPILER\n").await;
}

#[tokio::test]
async fn conda_flang_activate() {
    // flang-activation-feedstock: recipe/activate.sh
    run_conda_script(
        r#"export CONDA_BACKUP_FC=$FC
export FC="x86_64-conda-linux-gnu-flang"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_flang_deactivate() {
    // flang-activation-feedstock: recipe/deactivate.sh
    run_conda_script("export FC=$CONDA_BACKUP_FC\n").await;
}

// === Rust activation (uses [[ ]] and mkdir) ===

#[tokio::test]
async fn conda_rust_activate() {
    // rust-activation-feedstock: recipe/activate.sh (preprocessed)
    // This tests [[ ]] conditionals, ${VAR:-default}, and complex variable exports
    run_conda_script(
        r#"#!/usr/bin/env bash

export CARGO_HOME=${CARGO_HOME:-${CONDA_PREFIX}/.cargo}
export CARGO_CONFIG=${CARGO_CONFIG:-${CARGO_HOME}/config}
export RUSTUP_HOME=${RUSTUP_HOME:-${CARGO_HOME}/rustup}

[[ -d ${CARGO_HOME} ]] || mkdir -p ${CARGO_HOME}

export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=${CC_FOR_BUILD:-${CONDA_PREFIX}/bin/x86_64-conda-linux-gnu-cc}
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=${CC:-${CONDA_PREFIX}/bin/x86_64-conda-linux-gnu-cc}
export CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
export CONDA_RUST_HOST=X86_64_UNKNOWN_LINUX_GNU
export CONDA_RUST_TARGET=X86_64_UNKNOWN_LINUX_GNU
export PKG_CONFIG_PATH_X86_64_UNKNOWN_LINUX_GNU=${CONDA_PREFIX}/lib/pkgconfig
export PKG_CONFIG_PATH_X86_64_UNKNOWN_LINUX_GNU=${PREFIX:-${CONDA_PREFIX}}/lib/pkgconfig

export CC_x86_64_unknown_linux_gnu="${CC_FOR_BUILD:-${CONDA_PREFIX}/bin/x86_64-conda-linux-gnu-cc}"
export CFLAGS_x86_64_unknown_linux_gnu="-isystem ${CONDA_PREFIX}/include"
export CFLAGS_x86_64_unknown_linux_gnu="${CFLAGS}"
export CPPFLAGS_x86_64_unknown_linux_gnu="-isystem ${CONDA_PREFIX}/include"
export CPPFLAGS_x86_64_unknown_linux_gnu="${CPPFLAGS}"
export CXXFLAGS_x86_64_unknown_linux_gnu="-isystem ${CONDA_PREFIX}/include"
export CXXFLAGS_x86_64_unknown_linux_gnu="${CXXFLAGS}"

if [[ "linux-64" == linux*  ]]; then
  export CARGO_BUILD_RUSTFLAGS="-C link-arg=-Wl,-rpath-link,${PREFIX:-${CONDA_PREFIX}}/lib -C link-arg=-Wl,-rpath,${PREFIX:-${CONDA_PREFIX}}/lib"
elif [[ "linux-64" == win* ]]; then
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=${CONDA_PREFIX}/bin/lld-link
  export AR_x86_64_unknown_linux_gnu="${AR}"
  export AR_x86_64_unknown_linux_gnu=$CONDA_PREFIX/bin/llvm-lib
  export CC_x86_64_unknown_linux_gnu=$CONDA_PREFIX/bin/clang-cl
  export CXX_x86_64_unknown_linux_gnu=$CONDA_PREFIX/bin/clang-cl
  export LDFLAGS="$LDFLAGS -manifest:no"
  export CMAKE_GENERATOR=Ninja
elif [[ "linux-64" == osx* ]]; then
  export CARGO_BUILD_RUSTFLAGS="-C link-arg=-Wl,-rpath,${PREFIX:-${CONDA_PREFIX}}/lib"
  if [[ "${CONDA_BUILD:-}" != "" ]]; then
    export CARGO_BUILD_RUSTFLAGS="$CARGO_BUILD_RUSTFLAGS -C link-arg=-Wl,-headerpad_max_install_names -C link-arg=-Wl,-dead_strip_dylibs"
  fi
fi

export PATH=${CARGO_HOME}/bin:${PATH}

if [[ "linux-64" == osx* || "linux-64" == linux* ]]; then
  if [[ "X86_64_UNKNOWN_LINUX_GNU" != "X86_64_UNKNOWN_LINUX_GNU" ]]; then
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=-Wl,-rpath,${BUILD_PREFIX:-${CONDA_PREFIX}}/lib"
  fi
fi
"#,
    )
    .await;
}

// === Compiler activation scripts with functions and complex shell features ===

#[tokio::test]
async fn conda_ctng_gcc_activate_function_definition() {
    // ctng-compiler-activation-feedstock uses function definitions and
    // ${BASH_SOURCE[0]} array access. Test that the function keyword + parens works.
    run_conda_script(
        r#"_get_sourced_filename() {
    if [ -n "${BASH_SOURCE+x}" ]; then
        basename "${BASH_SOURCE}"
    else
        echo "UNKNOWN"
    fi
}
echo "$(_get_sourced_filename)"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_clang_activate_function_keyword() {
    // clang-compiler-activation-feedstock uses `function name() {` syntax
    run_conda_script(
        r#"function _get_sourced_filename() {
    if [ -n "${BASH_SOURCE+x}" ]; then
        basename "${BASH_SOURCE}"
    else
        echo "UNKNOWN"
    fi
}
echo "$(_get_sourced_filename)"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_tc_activation_pattern() {
    // The _tc_activation function pattern from ctng/clang compiler activation scripts.
    // Tests local variables, for loops, if/elif/else, export, and unset.
    run_conda_script(
        r#"_tc_activation() {
    local new_val
    for val in "$@"; do
        local var_name="${val%%,*}"
        new_val="${val#*,}"
        if [ -n "${!var_name+x}" ]; then
            export "CONDA_BACKUP_${var_name}=${!var_name}"
        fi
        export "${var_name}=${new_val}"
    done
}

_tc_activation \
    "CC,gcc" \
    "CXX,g++" \
    "CFLAGS,-O2"

echo $CC
echo $CXX
echo $CFLAGS
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_tc_deactivation_pattern() {
    // The _tc_deactivation function pattern - restores backed-up env vars
    run_conda_script(
        r#"_tc_deactivation() {
    for val in "$@"; do
        local var_name="${val}"
        local backup_var="CONDA_BACKUP_${var_name}"
        if [ -n "${!backup_var+x}" ]; then
            export "${var_name}=${!backup_var}"
            unset "${backup_var}"
        else
            unset "${var_name}"
        fi
    done
}

export CC=gcc
export CONDA_BACKUP_CC=old_gcc
_tc_deactivation CC
echo ${CC}
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_win_activation_env_setup() {
    // clang-win-activation-feedstock: activate-msvc-headers-libs.sh pattern
    // Tests dirname, readlink, and path manipulation
    run_conda_script(
        r#"export CONDA_PREFIX="/tmp/test_conda_prefix"

if [ -z "${VSINSTALLDIR+x}" ]; then
    export VSINSTALLDIR="${CONDA_PREFIX}/vs_default"
fi

export INCLUDE="${CONDA_PREFIX}/include"
export LIB="${CONDA_PREFIX}/lib"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_path_manipulation() {
    // Common pattern in activation scripts: prepending to PATH
    run_conda_script(
        r#"export CONDA_PREFIX="/tmp/test_conda_prefix"
export PATH="${CONDA_PREFIX}/bin:${PATH}"
echo "ok"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_conditional_platform_check() {
    // Pattern from multiple activation scripts: platform-dependent configuration
    run_conda_script(
        r#"target_platform="linux-64"
if [ "$target_platform" = "linux-64" ]; then
    export CONDA_BUILD_SYSROOT="/opt/conda/x86_64-conda-linux-gnu/sysroot"
elif [ "$target_platform" = "osx-64" ]; then
    export CONDA_BUILD_SYSROOT="/opt/conda/sysroot"
fi
echo $CONDA_BUILD_SYSROOT
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_backup_and_restore_pattern() {
    // Common pattern: backup current value, set new value, then restore
    run_conda_script(
        r#"export ORIGINAL_CC="old_compiler"
export CONDA_BACKUP_CC="$ORIGINAL_CC"
export CC="new_compiler"
echo "CC=$CC"
echo "backup=$CONDA_BACKUP_CC"
# Restore
export CC="$CONDA_BACKUP_CC"
unset CONDA_BACKUP_CC
echo "restored CC=$CC"
"#,
    )
    .await;
}

#[tokio::test]
async fn conda_pattern_suffix_remove() {
    // Test ${var%%pattern} - simplest case
    TestBuilder::new()
        .command(r#"val="hello.world.txt"; echo "${val%%.*}""#)
        .assert_stdout("hello\n")
        .run()
        .await;

    // Test ${var%%,*}
    TestBuilder::new()
        .command(r#"val="CC,gcc"; echo "${val%%,*}""#)
        .assert_stdout("CC\n")
        .run()
        .await;
}

#[tokio::test]
async fn conda_pattern_prefix_remove() {
    // Test ${var#pattern} and ${var##pattern}
    TestBuilder::new()
        .command(r#"val="CC,gcc"; echo "${val#*,}""#)
        .assert_stdout("gcc\n")
        .run()
        .await;
}

#[tokio::test]
async fn conda_check_set_modifier() {
    // Test ${var+word} - substitute word if var is set
    TestBuilder::new()
        .command(r#"MY_VAR="hello"; echo "${MY_VAR+x}""#)
        .assert_stdout("x\n")
        .run()
        .await;

    // Unset var should produce empty
    TestBuilder::new()
        .command(r#"unset NONEXISTENT_VAR; echo "${NONEXISTENT_VAR+x}""#)
        .assert_stdout("\n")
        .run()
        .await;
}

#[tokio::test]
async fn conda_indirect_expansion() {
    // Test ${!var}
    TestBuilder::new()
        .command(r#"MY_VAR="hello"; ref="MY_VAR"; echo "${!ref}""#)
        .assert_stdout("hello\n")
        .run()
        .await;
}

#[tokio::test]
async fn conda_indirect_with_check_set() {
    // Test ${!var+x} - indirect + check-if-set
    TestBuilder::new()
        .command(r#"MY_VAR="hello"; ref="MY_VAR"; echo "${!ref+x}""#)
        .assert_stdout("x\n")
        .run()
        .await;
}

#[tokio::test]
async fn conda_if_compound_condition() {
    // Test if with compound conditions: if [ ... ] && [ ... ]; then
    TestBuilder::new()
        .command("if [ 1 = 1 ] && [ 2 = 2 ]; then echo yes; else echo no; fi")
        .assert_stdout("yes\n")
        .run()
        .await;

    TestBuilder::new()
        .command("if [ 1 = 2 ] && [ 2 = 2 ]; then echo yes; else echo no; fi")
        .assert_stdout("no\n")
        .run()
        .await;

    // Test elif
    TestBuilder::new()
        .command(r#"FOO=2; if [[ $FOO == 1 ]]; then echo "one"; elif [[ $FOO -eq 2 ]]; then echo "two"; fi"#)
        .assert_stdout("two\n")
        .run()
        .await;
}
