`single-use-variable` marks its fix unsafe when the value calls something and the next statement first formats or
unpacks a name, or reads a name declared `global` or `nonlocal`, since the call could change what that read sees.
