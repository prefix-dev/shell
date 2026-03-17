# eval builtin - basic
> eval echo hello
hello

# eval with variable expansion
> export X=world
> eval "echo $X"
world

# eval with command construction
> export CMD=echo
> eval "$CMD testing"
testing

# shift builtin inside function
> show() { echo $1; shift; echo $1; }
> show a b
a
b

# shift by N inside function
> skip() { echo $1; shift 2; echo $1; }
> skip x y z
x
z

# local sets a variable inside a function
> f() { local Y=localval; echo $Y; }
> f
localval

# Brace group basic
> { echo hello; echo world; }
hello
world

# Brace group with variable
> export V=test
> { echo $V; }
test
