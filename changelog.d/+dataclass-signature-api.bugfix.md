`dataclass-frozen`, `dataclass-kw-only` and `dataclass-slots` leave a public dataclass alone when a public signature of
the library returns it, as they do a dataclass its `__all__` lists.
