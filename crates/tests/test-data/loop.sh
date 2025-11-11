> for x in 1 2 3; do
>   echo $x
> done
1
2
3

> for item in apple banana orange; do
>     echo "Current fruit: $item"
> done
Current fruit: apple
Current fruit: banana
Current fruit: orange

> for item in "apple banana orange"; do
>     echo "Current fruit: $item"
> done
Current fruit: apple banana orange

# test single line for loop
> for item in "apple" "banana" "orange"; do echo "Current fruit: $item"; done
Current fruit: apple
Current fruit: banana
Current fruit: orange

> for item in a b c
> do
>     echo "Current letter: $item"
> done
Current letter: a
Current letter: b
Current letter: c

> i=0; while [[ $i -lt 5 ]]; do echo "Number: $i"; i=$((i+1)); done
Number: 0
Number: 1
Number: 2
Number: 3
Number: 4

> i=0; until [[ $i -gt 5 ]]; do echo "Number: $i"; i=$((i+1)); done
Number: 0
Number: 1
Number: 2
Number: 3
Number: 4
Number: 5

# > for i in {1..5}; do echo $i; done
# 1
# 2
# 3
# 4
# 5

# Test break in for loop
> for i in 1 2 3 4 5; do
>     if [[ $i == 3 ]]; then
>         break
>     fi
>     echo $i
> done
1
2

# Test continue in for loop
> for i in 1 2 3 4 5; do
>     if [[ $i == 3 ]]; then
>         continue
>     fi
>     echo $i
> done
1
2
4
5

# Test break in nested loop
> for i in 1 2 3; do
>     echo "outer: $i"
>     for j in a b c; do
>         echo "  inner: $j"
>         if [[ $j == b ]]; then
>             break
>         fi
>     done
> done
outer: 1
  inner: a
  inner: b
outer: 2
  inner: a
  inner: b
outer: 3
  inner: a
  inner: b