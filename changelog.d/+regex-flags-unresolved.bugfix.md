`unnamed-regex-group` (SMP532) and `unexplained-regex` (SMP533) no longer read a pattern as non-verbose when its `flags`
argument is a name that holds `re.VERBOSE`, so `#` comments are checked as comments and the fix no longer writes `(?:`
inside one.
