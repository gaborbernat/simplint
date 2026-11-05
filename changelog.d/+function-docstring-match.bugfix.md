`function-docstring` (SMP901) reads the functions in a `match` statement's `case` bodies, at module and class level, as
it already reads those in `if` and `try` blocks, so a docstring on an internal one is reported.
