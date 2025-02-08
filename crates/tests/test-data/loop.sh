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

# > for i in {1..5}; do echo $i; done
# 1
# 2
# 3
# 4
# 5