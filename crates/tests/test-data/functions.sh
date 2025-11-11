# Test basic function definition and call
> greet() {
>     echo "Hello, World!"
> }
> greet
Hello, World!

# Test function with parameters
> show_params() {
>     echo "First: $1"
>     echo "Second: $2"
> }
> show_params "foo" "bar"
First: foo
Second: bar

# Test function with multiple parameters
> add_values() {
>     echo "A=$1, B=$2, C=$3"
> }
> add_values "10" "20" "30"
A=10, B=20, C=30

# Test function keyword syntax
> function my_function {
>     echo "Using function keyword"
> }
> my_function
Using function keyword

# Test function keyword with parentheses
> function another_function() {
>     echo "Function with parens"
> }
> another_function
Function with parens

# Test multiple functions in sequence
> myfunc1() {
>     echo "First function"
> }
> myfunc2() {
>     echo "Second function"
> }
> myfunc1
> myfunc2
First function
Second function

# Test function overriding
> test_override() {
>     echo "First version"
> }
> test_override
> test_override() {
>     echo "Second version"
> }
> test_override
First version
Second version

# Test function with multiple commands
> multi_cmd() {
>     echo "Line 1"
>     echo "Line 2"
>     echo "Line 3"
> }
> multi_cmd
Line 1
Line 2
Line 3

# Test function with variable expansion
> greet_name() {
>     echo "Hello, $1!"
>     echo "Welcome, $1"
> }
> greet_name "Alice"
Hello, Alice!
Welcome, Alice

# Test function with empty parameters
> test_empty() {
>     echo "A=$1"
>     echo "B=$2"
> }
> test_empty
A=
B=

# Test 'which' command with functions
> myfunc() {
>     echo "test"
> }
> which myfunc
<user function>

# Test function with exported variable
> export_test() {
>     export MY_VAR="exported"
>     echo "Set MY_VAR"
> }
> export_test
> echo $MY_VAR
Set MY_VAR
exported

# Test function with local variable scope (basic)
> set_local() {
>     FOO="local value"
>     echo "Inside: $FOO"
> }
> FOO="global"
> set_local
> echo "Outside: $FOO"
Inside: local value
Outside: local value

# Test function calling built-in commands
> use_builtins() {
>     pwd > /dev/null
>     echo "Working with builtins"
> }
> use_builtins
Working with builtins

# Test function with command substitution in parameters
> echo_twice() {
>     echo "$1 $1"
> }
> echo_twice "$(echo 'hello')"
hello hello

# Real-world example: Simple logger function
> log() {
>     echo "[LOG] $1"
> }
> log "Application started"
> log "Processing data"
[LOG] Application started
[LOG] Processing data

# Real-world example: Error handling wrapper
> run_with_msg() {
>     echo "Running: $1"
>     echo "Status: $2"
> }
> run_with_msg "backup.sh" "success"
Running: backup.sh
Status: success

# Real-world example: Path manipulation
> make_path() {
>     echo "$1/$2"
> }
> make_path "/home/user" "documents"
/home/user/documents

# Test function name that's not a reserved word
> my_custom_func() {
>     echo "Custom function works"
> }
> my_custom_func
Custom function works
