`single-use-variable` (SMP504) decides on the layout a value takes once `ruff format` has run, so a second run after
`ruff format` no longer inlines more. A value the source spreads over lines is inlined when the formatter would join it,
into an f-string before Python 3.12 too, a value in parentheses the formatter drops is measured without them, and a
value assigned before a semicolon is inlined into the statement after it.
