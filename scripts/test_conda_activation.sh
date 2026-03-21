#!/usr/bin/env bash
# Test conda-forge activation scripts against prefix-dev/shell
#
# This script:
# 1. Downloads activation/deactivation .sh scripts from conda-forge feedstocks on GitHub
# 2. Preprocesses conda-build template variables (@VAR@) with dummy values
# 3. Runs each script through the shell binary
# 4. Reports which scripts pass/fail (parsing + execution)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SHELL_BIN="${ROOT_DIR}/target/release/shell"
DOWNLOAD_DIR="${ROOT_DIR}/target/conda_activation_scripts"
RESULTS_DIR="${ROOT_DIR}/target/conda_activation_results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Check shell binary exists
if [ ! -f "$SHELL_BIN" ]; then
    echo -e "${RED}Error: Shell binary not found at $SHELL_BIN${NC}"
    echo "Build it first with: cargo build --release"
    exit 1
fi

mkdir -p "$DOWNLOAD_DIR" "$RESULTS_DIR"

# Define all conda-forge activation script URLs
# Format: "repo_name:branch:path_in_repo"
SCRIPTS=(
    # go-activation
    "go-activation-feedstock:main:recipe/activate.sh"
    "go-activation-feedstock:main:recipe/deactivate.sh"

    # rust-activation
    "rust-activation-feedstock:main:recipe/activate.sh"

    # flang-activation
    "flang-activation-feedstock:main:recipe/activate.sh"
    "flang-activation-feedstock:main:recipe/deactivate.sh"

    # ctng-compiler-activation (gcc, g++, gfortran)
    "ctng-compiler-activation-feedstock:main:recipe/activate-gcc.sh"
    "ctng-compiler-activation-feedstock:main:recipe/deactivate-gcc.sh"
    "ctng-compiler-activation-feedstock:main:recipe/activate-g++.sh"
    "ctng-compiler-activation-feedstock:main:recipe/deactivate-g++.sh"
    "ctng-compiler-activation-feedstock:main:recipe/activate-gfortran.sh"
    "ctng-compiler-activation-feedstock:main:recipe/deactivate-gfortran.sh"

    # clang-compiler-activation
    "clang-compiler-activation-feedstock:main:recipe/activate-clang.sh"
    "clang-compiler-activation-feedstock:main:recipe/deactivate-clang.sh"
    "clang-compiler-activation-feedstock:main:recipe/activate-clang++.sh"
    "clang-compiler-activation-feedstock:main:recipe/deactivate-clang++.sh"

    # clang-win-activation (sh files only)
    "clang-win-activation-feedstock:main:recipe/activate-clang_win-64.sh"
    "clang-win-activation-feedstock:main:recipe/activate-clangxx_win-64.sh"
    "clang-win-activation-feedstock:main:recipe/activate-msvc-headers-libs.sh"
    "clang-win-activation-feedstock:main:recipe/activate-winsdk.sh"
    "clang-win-activation-feedstock:main:recipe/deactivate-clang_win-64.sh"
    "clang-win-activation-feedstock:main:recipe/deactivate-clangxx_win-64.sh"
    "clang-win-activation-feedstock:main:recipe/deactivate-msvc-headers-libs.sh"
    "clang-win-activation-feedstock:main:recipe/deactivate-winsdk.sh"
)

echo -e "${CYAN}=== Conda-Forge Activation Script Test Runner ===${NC}"
echo -e "${CYAN}Shell binary: ${SHELL_BIN}${NC}"
echo ""

# Step 1: Download all scripts
echo -e "${CYAN}--- Downloading activation scripts ---${NC}"
for entry in "${SCRIPTS[@]}"; do
    IFS=':' read -r repo branch path <<< "$entry"
    filename="${repo}__$(basename "$path")"
    dest="${DOWNLOAD_DIR}/${filename}"

    if [ -f "$dest" ]; then
        echo "  [cached] $filename"
    else
        url="https://raw.githubusercontent.com/conda-forge/${repo}/${branch}/${path}"
        if curl -sS -f -o "$dest" "$url" 2>/dev/null; then
            echo "  [downloaded] $filename"
        else
            echo -e "  ${YELLOW}[not found] $filename (URL: $url)${NC}"
            rm -f "$dest"
        fi
    fi
done
echo ""

# Step 2: Preprocess template variables (@VAR@) with dummy values
# Conda-build replaces these at package build time; we substitute with plausible defaults
preprocess_script() {
    local input="$1"
    local output="$2"

    sed \
        -e 's/@CHOST@/x86_64-conda-linux-gnu/g' \
        -e 's/@CONDA_TOOLCHAIN_HOST@/x86_64-conda-linux-gnu/g' \
        -e 's/@CONDA_TOOLCHAIN_BUILD@/x86_64-conda-linux-gnu/g' \
        -e 's/@MACH@/x86_64/g' \
        -e 's/@VENDOR@/conda/g' \
        -e 's/@OS@/linux/g' \
        -e 's/@cross_target_platform@/linux-64/g' \
        -e 's/@native_compiler_subdir@/linux-64/g' \
        -e 's/@target_platform@/linux-64/g' \
        -e 's/@build_platform@/linux-64/g' \
        -e 's/@c_compiler@/gcc/g' \
        -e 's/@cxx_compiler@/g++/g' \
        -e 's/@fortran_compiler@/gfortran/g' \
        -e 's/@c_compiler_version@/12/g' \
        -e 's/@cxx_compiler_version@/12/g' \
        -e 's/@fortran_compiler_version@/12/g' \
        -e 's/@CC@/gcc/g' \
        -e 's/@CXX@/g++/g' \
        -e 's/@FC@/gfortran/g' \
        -e 's/@IS_WIN@/False/g' \
        -e 's/@CMAKE_SYSTEM_NAME@/Linux/g' \
        -e 's/@CONDA_BUILD_CROSS_COMPILATION@/0/g' \
        -e 's|@CONDA_BUILD_SYSROOT@|/opt/conda/x86_64-conda-linux-gnu/sysroot|g' \
        -e 's/@rust_arch@/x86_64-unknown-linux-gnu/g' \
        -e 's/@rust_arch_env@/X86_64_UNKNOWN_LINUX_GNU/g' \
        -e 's/@rust_arch_env_build@/X86_64_UNKNOWN_LINUX_GNU/g' \
        -e 's/@rust_default_cc@/x86_64-conda-linux-gnu-cc/g' \
        -e 's/@rust_default_cc_build@/x86_64-conda-linux-gnu-cc/g' \
        -e 's/@CONDA_RUST_HOST_LOWER@/x86_64_unknown_linux_gnu/g' \
        -e 's/@CONDA_RUST_TARGET_LOWER@/x86_64_unknown_linux_gnu/g' \
        -e 's/@GOARCH@/amd64/g' \
        -e 's/@GOOS@/linux/g' \
        -e 's/@CGO_ENABLED@/1/g' \
        -e 's/@MACOSX_SDK_VERSION@/10.15/g' \
        -e 's/@[A-Za-z_][A-Za-z_0-9]*@/TEMPLATE_VALUE/g' \
        "$input" > "$output"
}

# Step 3: Run each script through the shell
echo -e "${CYAN}--- Running activation scripts through shell ---${NC}"
echo ""

for script_file in "$DOWNLOAD_DIR"/*.sh; do
    [ -f "$script_file" ] || continue

    filename="$(basename "$script_file")"
    preprocessed="${RESULTS_DIR}/${filename}.preprocessed"
    result_file="${RESULTS_DIR}/${filename}.result"

    TOTAL=$((TOTAL + 1))

    # Preprocess template variables
    preprocess_script "$script_file" "$preprocessed"

    # Set up a minimal environment that activation scripts expect
    export CONDA_PREFIX="/tmp/test_conda_prefix"
    export PREFIX="/tmp/test_conda_prefix"
    export BUILD_PREFIX="/tmp/test_conda_build_prefix"
    export CONDA_BUILD=""
    export PATH="/usr/local/bin:/usr/bin:/bin"

    # Run through the shell binary with a timeout
    # We add 'true' at the end so that the script succeeds even if last command is benign failure
    # We wrap in a subshell command string using source-like behavior
    script_content=$(cat "$preprocessed")

    # Run the preprocessed script through the shell
    set +e
    output=$("$SHELL_BIN" -c "$script_content" 2>"${result_file}.stderr" )
    exit_code=$?
    set -e

    stderr_content=""
    if [ -f "${result_file}.stderr" ]; then
        stderr_content=$(cat "${result_file}.stderr")
    fi

    # Store results
    echo "exit_code=$exit_code" > "$result_file"
    echo "stdout=$output" >> "$result_file"
    echo "stderr=$stderr_content" >> "$result_file"

    # Report
    if [ $exit_code -eq 0 ]; then
        echo -e "  ${GREEN}[PASS]${NC} $filename"
        PASSED=$((PASSED + 1))
    else
        echo -e "  ${RED}[FAIL]${NC} $filename (exit code: $exit_code)"
        if [ -n "$stderr_content" ]; then
            # Show first 3 lines of stderr
            echo "$stderr_content" | head -3 | while IFS= read -r line; do
                echo -e "         ${RED}$line${NC}"
            done
        fi
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo -e "${CYAN}=== Results ===${NC}"
echo -e "  Total:   $TOTAL"
echo -e "  ${GREEN}Passed:  $PASSED${NC}"
echo -e "  ${RED}Failed:  $FAILED${NC}"
echo -e "  ${YELLOW}Skipped: $SKIPPED${NC}"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${YELLOW}--- Failed script details ---${NC}"
    for result_file in "$RESULTS_DIR"/*.sh.result; do
        [ -f "$result_file" ] || continue
        exit_code=$(grep "^exit_code=" "$result_file" | cut -d= -f2)
        if [ "$exit_code" != "0" ]; then
            filename="$(basename "$result_file" .result)"
            echo ""
            echo -e "  ${RED}$filename${NC}:"
            echo "    Preprocessed script: ${RESULTS_DIR}/${filename}.preprocessed"
            if [ -f "${result_file}.stderr" ]; then
                echo "    Stderr:"
                cat "${result_file}.stderr" | head -10 | sed 's/^/      /'
            fi
        fi
    done
    echo ""
    exit 1
else
    echo -e "${GREEN}All activation scripts passed!${NC}"
    exit 0
fi
