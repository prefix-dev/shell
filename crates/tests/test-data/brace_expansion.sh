# Basic comma-separated list
> echo {a,b,c}
a b c

# Numeric sequence
> echo {1..5}
1 2 3 4 5

# Character sequence
> echo {a..e}
a b c d e

# Reverse numeric sequence
> echo {5..1}
5 4 3 2 1

# Reverse character sequence
> echo {e..a}
e d c b a

# Numeric sequence with step
> echo {1..10..2}
1 3 5 7 9

# Numeric sequence with step (reverse)
> echo {10..1..2}
10 8 6 4 2

# Empty elements in list
> echo {a,,b}
a  b

# Quoted braces - should not expand
> echo "{a,b,c}"
{a,b,c}

# Mixed with other arguments
> echo start {a,b,c} end
start a b c end
