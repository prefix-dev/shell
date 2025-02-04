# Basic variable expansion
> export FOO="hello"
> echo "${FOO}"
hello

# Default value tests
> unset UNSET_VAR
> echo "${UNSET_VAR:-default}"
default
> echo "${EMPTY_VAR:-not empty}"
not empty

# Alternate value tests
> export SET_VAR="value"
> echo "${SET_VAR:+alternate}"
alternate
> echo "${UNSET_VAR:+alternate}"
%empty

# Assign default tests
> echo "${ASSIGN_VAR:=default_value}"
> echo "$ASSIGN_VAR"
default_value
default_value

# Substring operations
> export LONG="Hello World"
> echo "${LONG:6}"
> echo "${LONG:0:5}"
World
Hello

# Empty vs unset
# > export EMPTY=""
# > echo "${EMPTY:-default}"
# default

# > export EMPTY=""
# > echo "${EMPTY-default}"
# %empty

# > unset EMPTY
# > echo "${EMPTY-default}"
# default

# > unset EMPTY
# > echo "${EMPTY-default}"
# default

# Multiple substitutions
> unset VAR1 VAR2
> echo "${VAR1:-one} ${VAR2:-two}"
one two

# # Complex defaults
# > echo "${UNDEFINED:-$(echo complex)}"
# complex

# Escape sequences in expansion
> echo "${FOO:-a\}b}"
a}b

# # Error cases
# > echo "${UNDEFINED?error message}"
# error message

> export VERSION="1.2.3"
> echo "Version: ${VERSION:2}"
Version: 2.3