# Note: bash and zsh are quite a bit different with brace expansion
# We follow the simpler bash rules (e.g. no expansion of variables in ranges)
> echo {1..10}
1 2 3 4 5 6 7 8 9 10

> echo {01..20}
01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20

> FOOBAR=5
> echo {1..$FOOBAR}
{1..5}

> {1..{1..5}}
{1..{1..5}}

> echo {1..x}
{1..x}

> echo {a..c}
a b c

> echo {1..10..2}
1 3 5 7 9

> echo {10..1..2}
10 8 6 4 2

> echo {10..1..-4}
10 6 2

> echo {0a..0c}
{0a..0c}

> echo {aa..ac}
{aa..ac}

> echo {001..10}
001 002 003 004 005 006 007 008 009 010

# If leading 0 are indicated, all numbers will have leading 0 up to the maximum digits
> echo {01..100}
001 002 003 004 005 006 007 008 009 010 ...