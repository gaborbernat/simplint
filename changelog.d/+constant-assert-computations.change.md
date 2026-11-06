`constant-assert` (SMP804) also reports a `format` call or `%` expression on a string literal whose text lies outside
its fields, and a computation on literals that is truthy, such as `1 + 1 == 2`. An assertion on a generator expression,
which is always truthy, now gets an unsafe fix that wraps it in `all()`, unless the generator is asynchronous.
