[`missing-from-all`](../src/rules/api/missing_from_all.md) (SMP401) no longer reports a name that only tests import when
production code loads its module as a value, as `{"compat": compat}` does. The fix would have listed the name in
`__all__`.
