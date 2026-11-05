`module-variable-case` (SMP102) no longer renames a `type` alias. The statement binds a type rather than a value, so
`type vector = list[float]` keeps its name.
