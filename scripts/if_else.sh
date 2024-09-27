FOO=2
if [[ $FOO -eq 1 ]]; then
    echo "FOO is 1"
elif [[ $FOO -eq 2 ]]; then
    echo "FOO is 2"
else
    echo "FOO is not 1 or 2"
fi
