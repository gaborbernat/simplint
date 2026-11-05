`rst-returns` (SMP923) reads a quoted return annotation, such as `-> "None"` or `-> 'NoReturn'`, as the unquoted form it
spells. Before, a quoted `None` counted as returning a value.
