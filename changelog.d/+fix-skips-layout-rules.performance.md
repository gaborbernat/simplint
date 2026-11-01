`simplint check --fix` runs Ruff's findings checks without the whitespace, blank line and line length rules, whose
findings it ignores, unless a file's suppression comments may name one of them; the fix loop on `sympy` does about 6%
less work.
