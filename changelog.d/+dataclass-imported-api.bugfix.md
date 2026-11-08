`dataclass-frozen`, `dataclass-kw-only` and `dataclass-slots` leave a dataclass alone when a package without `__all__`,
or with a dynamic one, re-exports it and another file imports it from the package, as the other public API rules do.
