# Basic heredoc
> cat <<EOF
> hello world
> EOF
hello world

# Multi-line heredoc
> cat <<EOF
> line one
> line two
> line three
> EOF
line one
line two
line three

# Heredoc with variable expansion
> NAME="World"
> cat <<EOF
> Hello $NAME!
> EOF
Hello World!

# Quoted heredoc (no expansion)
> NAME="World"
> cat <<'EOF'
> Hello $NAME!
> EOF
Hello $NAME!

# Heredoc in a function
> greet() {
>     cat <<EOF
> Hello from function
> EOF
> }
> greet
Hello from function

# Heredoc with command after
> cat <<EOF
> first
> EOF
> echo "after heredoc"
first
after heredoc
