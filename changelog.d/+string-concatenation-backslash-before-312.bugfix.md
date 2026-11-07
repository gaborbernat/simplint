`string-concatenation` (SMP503) no longer reports a chain whose non-literal operand holds a string with a backslash when
the target is older than Python 3.12, where no f-string replacement field can contain one.
