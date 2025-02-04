# Test string equality
> if [[ "hello" == "hello"]]; then echo true; else echo false; fi
true

> if [[ "hello" == "world"]]; then echo true; else echo false; fi
false

# Test string starts/ends with (does not work yet)
# > if [[ "hello world" == hello* ]]; then echo true; else echo false; fi
# true

# > if [[ "hello world" == *world ]]; then echo true; else echo false; fi
# true

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