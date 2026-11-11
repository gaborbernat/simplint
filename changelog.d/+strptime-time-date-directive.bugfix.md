[`up-strptime-date`](../src/rules/logic/up_strptime_date.md) makes the `.time()` fix unsafe when the format has a date
directive, and the `.date()` and `.time()` fixes unsafe when it has a seconds directive. A safe `--fix` used to rewrite
`datetime.strptime(text, "%m-%d %H:%M").time()` into a call that accepts February 29 in a year without a leap day, which
the original call rejects with `ValueError`, and `datetime.strptime(text, "%H:%M:%S").time()` into a call that accepts
second 60.
