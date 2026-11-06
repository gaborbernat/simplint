`all-format` (SMP405) no longer reports a list whose closing bracket follows a line break with no trailing comma when
the project's Ruff configuration selects [COM812](https://docs.astral.sh/ruff/rules/missing-trailing-comma/), which
reports it. A list whose last name is parenthesized and followed by a comma is no longer reported as missing that comma.
