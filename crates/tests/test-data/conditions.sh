# Test string equality
> if [[ "hello" == "hello"]]; then echo true; else echo false; fi
true

> if [[ "hello" == "world"]]; then echo true; else echo false; fi
false

# Test string starts/ends with (does not work yet)
> if [[ "hello world" == hello* ]]; then echo true; else echo false; fi
true

> if [[ "hello world" == *world ]]; then echo true; else echo false; fi
true

# should not match because glob is quoted
> if [[ "hello world" == "*world" ]]; then echo true; else echo false; fi
false

> if [[ "*world" == "*world" ]]; then echo true; else echo false; fi
true

# Test more complex glob patterns
> if [[ "hello.txt" == *.txt ]]; then echo true; else echo false; fi
true

> if [[ "hello.txt" == h*.txt ]]; then echo true; else echo false; fi
true

> if [[ "hello.txt" == h??lo.txt ]]; then echo true; else echo false; fi
true

# Test multiple wildcards
> if [[ "hello world test" == h*d* ]]; then echo true; else echo false; fi
true

> if [[ "abc123xyz" == *123* ]]; then echo true; else echo false; fi
true

# Test pattern at start/middle/end
> if [[ "testing123" == test* ]]; then echo true; else echo false; fi
true

> if [[ "testing123" == *ing* ]]; then echo true; else echo false; fi
true

> if [[ "testing123" == *123 ]]; then echo true; else echo false; fi
true

# Test exact matches with wildcards present
> if [[ "*star" == "*star" ]]; then echo true; else echo false; fi
true

> if [[ "star*" == "star*" ]]; then echo true; else echo false; fi
true

# Test empty strings with patterns
> if [[ "" == * ]]; then echo true; else echo false; fi
true

> if [[ "" == ** ]]; then echo true; else echo false; fi
true

# Test quoted vs unquoted patterns
> if [[ "star*star" == "star*star" ]]; then echo true; else echo false; fi
true

> if [[ "star*star" == star*star ]]; then echo true; else echo false; fi
true

# Test mixed literal and pattern matching
> if [[ "hello.txt.old" == hello.*old ]]; then echo true; else echo false; fi
true

> if [[ "config.2024.json" == config.*.json ]]; then echo true; else echo false; fi
true

# Test case sensitivity
> if [[ "HELLO.txt" == hello.* ]]; then echo true; else echo false; fi
false

> if [[ "Hello.TXT" == *.txt ]]; then echo true; else echo false; fi
false

# Test numeric comparisons
> if [[ 5 -gt 3 ]]; then echo true; else echo false; fi
true

> if [[ 2 -lt 1 ]]; then echo true; else echo false; fi
false

# Test empty strings
> if [[ -z "" ]]; then echo true; else echo false; fi
true

> if [[ -n "hello" ]]; then echo true; else echo false; fi
true

# Test file existence (works locally)
# > touch testfile
# > if [[ -f testfile ]]; then echo true; else echo false; fi
# true

# works, but only on Unix systems
# > if [[ -d /tmp ]]; then echo true; else echo false; fi
# true

# Test variable existence
> if [[ -v PATH ]]; then echo true; else echo false; fi
true

> if [[ -v NONEXISTENT ]]; then echo true; else echo false; fi
false

# # Test AND/OR conditions
# > if [[ 1 == 1 && 2 == 2 ]]; then echo true; else echo false; fi
# true

# > if [[ 1 -eq 2 || 2 -eq 2 ]]; then echo true; else echo false; fi
# true