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

# Empty heredoc produces no output
> cat <<EOF
> EOF
> echo "after empty"
after empty

# Heredoc with different delimiter
> cat <<MYDELIM
> custom delimiter
> MYDELIM
custom delimiter

# Heredoc with special characters
> cat <<'EOF'
> angle <brackets> and pipes | and amps &
> semicolons; and parens ()
> EOF
angle <brackets> and pipes | and amps &
semicolons; and parens ()

# Heredoc with command substitution expansion
> cat <<EOF
> today is $(echo Tuesday)
> EOF
today is Tuesday

# Multiple heredocs in sequence
> cat <<EOF
> first heredoc
> EOF
> cat <<END
> second heredoc
> END
first heredoc
second heredoc

# Heredoc with tab stripping (<<-)
> cat <<-EOF
> 	indented with tab
> 	another tabbed line
> EOF
indented with tab
another tabbed line

# Heredoc with multiple variable expansions
> X="hello"
> Y="world"
> cat <<EOF
> $X $Y
> ${X}/${Y}
> EOF
hello world
hello/world

# Double-quoted delimiter (same as single-quoted: no expansion)
> VAR="test"
> cat <<"EOF"
> no $VAR expansion
> EOF
no $VAR expansion
