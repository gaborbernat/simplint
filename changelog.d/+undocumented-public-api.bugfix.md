`undocumented-public-api` (SMP903) reports a public `type` statement that has no docstring, as it does for other public
variables. It also reads the public definitions in a `match` statement's `case` bodies, at module and class level, as it
already reads those in `if` and `try` blocks, so they are no longer skipped.
