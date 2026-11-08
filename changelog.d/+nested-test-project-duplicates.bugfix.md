Nested test projects, such as the `roots/test-*` directories of Sphinx's test suite, are now checked as projects of
their own: a name one of them defines is not used by another, and their test-module status, project kind and
documentation stay within them. A name the run's project reads is used in them, and an `.. automodule::` with
`:members:` documents the members Sphinx renders, so their names count as used and their docstrings stay. Their files
are test input, so the docstring rules and the export and naming visibility rules leave them alone, while the other
rules still check them.
