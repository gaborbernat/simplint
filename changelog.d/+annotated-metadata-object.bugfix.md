[`imprecise-annotation`](../src/rules/typing/imprecise_annotation.md) no longer reports `object` used as a value in the
metadata of an `Annotated` annotation, such as `Annotated[int, object()]`; `object` as the first argument is still
reported.
