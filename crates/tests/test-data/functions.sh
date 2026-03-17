# Basic function definition and call
> greet() { echo hello; }
> greet
hello

# Function with multiple commands
> multi() { echo first; echo second; }
> multi
first
second

# Function with arguments (positional parameters)
> say() { echo $1; }
> say hello
hello

# Multiple positional parameters
> show() { echo "$1 $2 $3"; }
> show a b c
a b c

# Argument count $#
> count() { echo $#; }
> count a b c
3

# $@ expands to all arguments
> all() { echo $@; }
> all x y z
x y z

# Function with no args has $# = 0
> noargs() { echo $#; }
> noargs
0

# Function calling another function
> inner() { echo inner; }
> outer() { inner; echo outer; }
> outer
inner
outer

# Function using regular variables from outer scope
> export X=world
> showvar() { echo $X; }
> showvar
world

# Function that sets a variable visible to caller
> setvar() { export MYVAR=hello; }
> setvar
> echo $MYVAR
hello

# Function overriding another function
> f() { echo first; }
> f
first

> f() { echo second; }
> f
second
