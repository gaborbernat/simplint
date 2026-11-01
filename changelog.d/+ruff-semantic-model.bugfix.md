The semantic model behind the rules is now Ruff's own, so names follow Python's scoping more closely: PEP 695 type
parameters and match captures resolve to their own scope, a star import no longer counts as a binding, and a class body
that annotates `type` without a value no longer hides the builtin.
