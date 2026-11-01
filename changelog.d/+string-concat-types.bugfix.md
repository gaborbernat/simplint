`string-concatenation` (SMP112) offers its fix only when every operand is surely a `str`, so it no longer turns `+` on
an object with its own `__add__` or `__radd__`, such as `Markup`, a path or a `PageRange`, into an f-string that formats
it instead.
