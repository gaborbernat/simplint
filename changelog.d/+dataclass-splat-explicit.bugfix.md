`dataclass-frozen` (SMP701) and `dataclass-slots` (SMP702) treat a `**` splat that sets `frozen` or `slots` to something
other than `True` as an explicit option, so the fix for the other option is marked unsafe, as it already was for a
keyword.
