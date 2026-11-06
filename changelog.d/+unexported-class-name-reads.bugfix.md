`unexported-public-name` (SMP403) no longer renames a function or class whose `__name__` or `__qualname__` the run
reads, as `helper.__name__` or `type(self).__name__` does, since the rename changes what the read returns.
