[`dataclass-kw-only`](../src/rules/classes/dataclass_kw_only.md) no longer rewrites `cls(...)` in an ordinary method,
where `cls` is some other value, and offers no fix for a dataclass another file calls through `getattr(module, "Name")`.
