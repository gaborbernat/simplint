`module-variable-case` (SMP102) leaves a `mixedCase` module variable it cannot rename to Ruff's N816 when the project's
Ruff configuration selects it and does not list the name in `pep8-naming` `ignore-names`. Renameable names and every
`lower_case` name are still reported here.
