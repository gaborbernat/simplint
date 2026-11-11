[`up-codecs-open`](../src/rules/logic/up_codecs_open.md) parenthesizes a walrus or a `yield` passed as `encoding`,
`errors` or `buffering`, so `codecs.open(path, "r", name := "utf-8")` no longer becomes an invalid `open()` call that
reverts the file.
