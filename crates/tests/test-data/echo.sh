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
> echo "${FOOBAR:-}" "${OTHER:-defaultbar}"
foobar defaultbar

> FOOBAR="foobar"
> echo "${FOOBAR}"
> echo $FOOBAR
foobar
foobar