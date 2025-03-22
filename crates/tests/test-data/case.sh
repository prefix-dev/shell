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
