> fruit="apple"
> case "$fruit" in
>    "apple")
>        echo "You chose Apple!"
>        ;;
>    "banana")
>        echo "You chose Banana!"
>        ;;
>    "orange")
>        echo "You chose Orange!"
>        ;;
>    *)
>        echo "Unknown fruit!"
>        ;;
> esac
You chose Apple!


> number=3
> case "$number" in
>     1)
>         echo "Number is one."
>         ;;
>     2|3|4)
>         echo "Number is between two and four."
>         ;;
>     *)
>         echo "Number is something else."
>         ;;
> esac
Number is between two and four.


> number=5
> case "$number" in
>     1)
>         echo "Number is one."
>         ;;
>     2|3|4)
>         echo "Number is between two and four."
>         ;;
>     *)
>         echo "Number is something else."
>         ;;
> esac
Number is something else.


> shape="circle"
> case "$shape" in
>     (circle)
>         echo "It's a circle!"
>         ;;
>     (square)
>         echo "It's a square!"
>         ;;
>     *)
>         echo "Unknown shape!"
>         ;;
> esac
It's a circle!

> filename="document.png"
> case "$filename" in
>     (*.txt)
>         echo "This is a text file."
>         ;;
>     (*.jpg|*.png)
>         echo "This is an image file."
>         ;;
>     (*)
>         echo "Unknown file type."
>         ;;
> esac
This is an image file.


> tempname="document.txt"
> filename="tempname"
> case "$filename" in
>     (tempname)
>         echo "This is a tempname."
>          ;;
>     (*.jpg|*.png)
>         echo "This is an image file."
>          ;;
>      (*)
>          echo "Unknown file type."
>          ;;
> esac
This is a tempname.

> letter="c"
> case "$letter" in
>     ([a-c])
>         echo "Letter is between A and C."
>         ;;
>     ([d-f])
>         echo "Letter is between D and F."
>         ;;
>     (*)
>         echo "Unknown letter."
>         ;;
> esac
Letter is between A and C.

> val="hello"
> case "$val" in hello) echo "matched" ;; *) echo "no match" ;; esac
matched

> val="world"
> case "$val" in hello) echo "matched" ;; *) echo "no match" ;; esac
no match

> val="~/.local"
> case "$val" in '~' | '~'/*) echo "tilde" ;; *) echo "other" ;; esac
tilde

# Empty case body (just ;;)
> case "x" in y) ;; *) echo "default" ;; esac
default

# Case with only default
> case "anything" in *) echo "always matches" ;; esac
always matches

# Case without default (no match = no output)
> case "x" in y) echo "y" ;; z) echo "z" ;; esac
> echo "done"
done

# Inline with multiple commands separated by ;
> case "a" in a) echo "first"; echo "second" ;; esac
first
second

# Case with command substitution in word
> val=$(echo hello)
> case "$val" in hello) echo "matched cmd sub" ;; esac
matched cmd sub

# Nested case statements
> outer="a"
> inner="x"
> case "$outer" in
>     a)
>         case "$inner" in
>             x) echo "a-x" ;;
>             *) echo "a-other" ;;
>         esac
>         ;;
>     *) echo "other" ;;
> esac
a-x
