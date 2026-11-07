`vars-call` (SMP604) no longer reports a mapping built from `vars()` in the same expression, such as `dict(vars(obj))`,
a dict comprehension over `vars(obj).items()` or `{**vars(obj), ...}`, since the attributes of an open-ended object have
no key type to name.
