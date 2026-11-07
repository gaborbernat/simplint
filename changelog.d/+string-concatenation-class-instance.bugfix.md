`string-concatenation` (SMP503) no longer reports a `+` chain with an instance of a class the module defines, such as
`Markup("<b>") + "<i>"`, since `+` runs the class's own `__add__` and an f-string would change what the code does.
