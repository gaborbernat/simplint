`unused-definition` (SMP409) no longer reports a private protocol or TypedDict class that the file never reads when the
project's Ruff configuration selects [PYI046](https://docs.astral.sh/ruff/rules/unused-private-protocol/) or
[PYI049](https://docs.astral.sh/ruff/rules/unused-private-typed-dict/), which reports it.
