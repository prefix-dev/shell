FOO=2
if [[ $FOO -eq 1 ]];
then
    echo "FOO is 1";
elif [[ $FOO -eq 2 ]];
then
    echo "FOO is 2";
else
    echo "FOO is not 1 or 2";
fi

FOO=2
if [[ $FOO -eq 1 ]]; then
    echo "FOO is 1"
elif [[ $FOO -eq 2 ]]; then
    echo "FOO is 2"
else
    echo "FOO is not 1 or 2"
fi

FOO=2
if [[ $FOO -eq 1 ]];
then
    echo "FOO is 1";
elif [[ $FOO -eq 2 ]];
then
    echo "FOO is 2";
else
    echo "FOO is not 1 or 2";
fi

FOO=2
if [[ $FOO -eq 1 ]]
then
    echo "FOO is 1";
elif [[ $FOO -eq 2 ]]
then
    echo "FOO is 2";
else
    echo "FOO is not 1 or 2";
fi


if test -n "${xml_catalog_files_libxml2:-}"; then
    export XML_CATALOG_FILES="${xml_catalog_files_libxml2}"
else
    unset XML_CATALOG_FILES
fi
unset xml_catalog_files_libxml2