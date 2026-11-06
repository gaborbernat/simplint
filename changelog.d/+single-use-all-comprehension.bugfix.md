`single-use-variable` (SMP504) no longer inlines a list comprehension whose elements call something into a call to `all`
or `any`. Ruff's C419 then wrote it as a generator, which stops at the first false element, so the later calls never
ran.
