`dataclass-frozen` (SMP701) no longer reports a dataclass written to after construction when the plain class it would
become is one Ruff rejects: B903 (`class-as-data-structure`) when the class has no base classes or class keywords, no
decorator besides the dataclass's, no method but private ones, and no factory default or `init=False` field, or PLR0913
(`too-many-arguments`) when its parameters, excluding `init=False` fields and `KW_ONLY`, exceed `max-args`. Those rules
were the only other way to satisfy the rule, so the two rules asked for contradictory code. Other dataclasses are still
reported.
