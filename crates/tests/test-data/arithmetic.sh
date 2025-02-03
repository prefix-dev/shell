# Test basic arithmetic
> echo $((2 + 5))
7

> echo $((10 - 3))
7

> echo $((3 * 4))
12

> echo $((15 / 3))
5

# Test operator precedence
> echo $((2 + 3 * 4))
14

# Fails
# > echo $(((2 + 3) * 4))
# 20

# Test with variables
> export NUM=5
> echo $((NUM + 3))
8

> export A=2
> export B=3
> echo $((A * B + 1))
7

# # Test increment/decrement NOT IMPLEMENTED YET!
# > export COUNT=1
# > echo $((COUNT++))
# 1
# > echo $COUNT
# 2

# > export X=5
# > echo $((--X))
# 4

# Test complex expressions
> export BASE=2
> echo $((BASE ** 3 + 1))
9

# Test division and modulo
> echo $((10 / 3))
3
> echo $((10 % 3))
1