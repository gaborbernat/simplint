[`unnamed-regex-group`](../src/rules/logic/unnamed_regex_group.md) (SMP532) and
[`unexplained-regex`](../src/rules/logic/unexplained_regex.md) now read a `#` comment in a scoped `(?x:...)` group as a
comment, so a parenthesis in it is no longer counted as a group. A `(?-x:...)` group under `re.VERBOSE` is read as not
verbose, so its `#` and parentheses count.
