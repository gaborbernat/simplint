[`up-object-sentinel`](../src/rules/logic/up_object_sentinel.md) judges a class attribute by the other bindings of its
class body, so `Config.MISSING` assigned twice in `Config` is no longer reported, while `Other.LIMIT = object()` is
reported even when the module binds `LIMIT` twice.
