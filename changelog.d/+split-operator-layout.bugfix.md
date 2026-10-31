The line length model no longer treats an augmented assignment such as `|=` as an operator `ruff format` moves to the
start of a line, and now does for the `not` of `not in`, so a fix is no longer kept when the line it makes stays too
long, or withdrawn when `ruff format` can split the line to fit.
