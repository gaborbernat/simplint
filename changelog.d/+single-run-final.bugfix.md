A second run of the documented pipeline no longer changes code: `single-user-constant` counts a comment that
`ruff format` moved inside the parentheses around a value as trailing it and handles a function body on the `def` line,
`dependency-import-guard` unwraps a one-line `try: import x` instead of breaking the syntax, and the fixes run again on
the code Ruff's cleanup leaves.
