`dataclass-slots` no longer offers a safe fix when code reaches the `__dict__` or a weak reference of an object of
unknown class, as `vars(obj)` or `weakref.ref(obj)` in a helper do, and withholds it when `weakref.ref(Release(...))` or
`vars(Release(...))` reach a new instance.
