# Test echoing
> echo "foobar"
foobar

> echo "foobar" "bazbar"
foobar bazbar

> echo "foobar" bazbar
foobar bazbar

> export FOOBAR="foobar"
> echo "${FOOBAR:-}"
foobar

> export FOOBAR="foobar"
> echo "${FOOBAR:-}" "${FOOBAR:-}"
foobar foobar

> if test -n "${xml_catalog_files_libxml2:-}"; then
>   echo "true"
> fi