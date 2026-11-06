`duplicate-code` (SMP530) reports six or more statements in a row that another module repeats, comparing their tokens,
so comments, layout, imports and docstrings do not count. Test modules and stubs are left out, and
`logic.min-duplicate-statements` sets how many statements a copy needs. It offers no fix.
