[`dataclass-kw-only`](../src/rules/classes/dataclass_kw_only.md) rewrites a positional call inside an annotation, as in
`Annotated[str, Doc("...")]`, and writes a positional walrus as `name=(name := value)`, where the fix used to make the
file invalid Python.
