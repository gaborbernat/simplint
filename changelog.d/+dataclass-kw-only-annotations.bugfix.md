[`dataclass-kw-only`](../src/rules/classes/dataclass_kw_only.md) rewrites a positional call inside an annotation, as in
`Annotated[str, Doc("...")]` or a string annotation, and writes a positional walrus as `name=(name := value)`, where the
fix used to make the file invalid Python. A class named in the metadata of `Annotated[...]` is a value, so it gets no
fix, and neither does one a concatenated string annotation names.
