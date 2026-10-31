Fixes are withdrawn when they make a line no `ruff format` layout can fit in more cases: the first line of a string over
several lines, a `def` line with empty parentheses, an operand after the operator of a split operation, comments moved
deeper, code under `# fmt: off` or `# fmt: skip` or in a file `ruff format` excludes, and a pragma comment that an added
annotation moves. Lines Ruff does not report, such as one ending in a URL, or code after a bracket the formatter splits,
no longer withdraw a fix.
