`dataclass-kw-only` (SMP703) no longer reports a dataclass that is part of a library's public API, nor the positional
calls of it in other files, since those callers would break when the fields became keyword-only.
