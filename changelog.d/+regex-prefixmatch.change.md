[`unnamed-regex-group`](../src/rules/logic/unnamed_regex_group.md) and
[`unexplained-regex`](../src/rules/logic/unexplained_regex.md) now also read the patterns of `re.prefixmatch()` calls,
the function Python 3.15 adds next to `re.match()`, so a prefixmatch pattern with unnamed groups or four unexplained
parts is reported like the other `re` functions.
