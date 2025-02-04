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

# > for i in {1..5}; do echo $i; done
# 1
# 2
# 3
# 4
# 5