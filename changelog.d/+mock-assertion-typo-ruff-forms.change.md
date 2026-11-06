`mock-assertion-typo` (SMP814) leaves the prefix-less assertion names Ruff's PGH005 reports, such as
`assert mock.not_called()`, to Ruff when the project's Ruff configuration selects it; with Ruff's preview on, names on a
line of their own too. Names under `not` or `and`, in call arguments, and every misspelling are still reported here.
