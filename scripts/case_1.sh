fruit="apple"

case "$fruit" in
    "apple")
        echo "You chose Apple!"
        ;;
    "banana")
        echo "You chose Banana!"
        ;;
    "orange")
        echo "You chose Orange!"
        ;;
    *)
        echo "Unknown fruit!"
        ;;
esac
