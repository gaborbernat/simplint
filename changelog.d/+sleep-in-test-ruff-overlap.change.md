`sleep-in-test` (SMP808) no longer reports an `await asyncio.sleep` that is the whole body of a `while` loop when the
project's Ruff configuration selects [ASYNC110](https://docs.astral.sh/ruff/rules/async-busy-wait/), which reports it.
