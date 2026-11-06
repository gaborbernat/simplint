`cognitive-complexity` (SMP529) reports a function whose cognitive complexity, as SonarSource defines it, is over the
limit. The score counts each branch, loop and handler more the deeper it is nested, so the rule reads the nesting that
Ruff's cyclomatic complexity check ignores. The limit is 20 by default and is set with `max-cognitive-complexity` under
`[tool.simplint.logic]`.
