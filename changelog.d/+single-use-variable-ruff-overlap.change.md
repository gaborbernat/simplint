`single-use-variable` (SMP504) leaves a name returned right after its assignment to Ruff's RET504 only when the
project's Ruff configuration selects it, and otherwise now reports it; with Ruff's preview on, a `yield from` of the
name is left to RUF070 when it is selected and writes the same code.
