`string-concatenation` (SMP503) no longer reports the message of a logging call when the project's Ruff configuration
selects [G003](https://docs.astral.sh/ruff/rules/logging-string-concat/), which reports it, nor two literals joined
across lines when it selects [ISC003](https://docs.astral.sh/ruff/rules/explicit-string-concatenation/), which reports
it.
