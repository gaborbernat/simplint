`string-concatenation` (SMP112) offers its fix only when the code shows every operand is a `str`, so it no longer turns
`+` on an object with its own `__add__` or `__radd__`, such as `Markup`, a path or a `PageRange`, into an f-string that
formats it instead. It reads that from literals, `str` methods, annotations, loops, the module's own functions and
classes, and common standard library values such as `os.sep` or `os.path.join`.
