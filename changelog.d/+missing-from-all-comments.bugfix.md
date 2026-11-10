[missing-from-all](../src/rules/api/missing_from_all.md) (SMP401) no longer drops a comment when it lists names after
the last element of `__all__`, or in an empty `__all__` that holds a comment, where it offers no fix. It keeps a
`# noqa: F401` on an import that binds a name the listing leaves out.
