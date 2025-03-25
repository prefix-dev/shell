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
