set -e
if [[ $(cat nonexistent.txt) ]]; then
    echo "This should not be printed"
else
    echo "This should be printed"
fi