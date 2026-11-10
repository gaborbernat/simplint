[function-docstring](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP901.html) drops `Params:` and
`Yield:` sections when it turns a docstring into a comment, as the reST docstring rules already read them, and keeps a
line such as `: name: text`, which reST does not parse as a field.
