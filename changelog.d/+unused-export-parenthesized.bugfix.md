[unused-export](../src/rules/api/unused_export.md) removes an unused name in parentheses from `__all__`, as in
`__all__ = "zeta", ("beta")`, without leaving a parenthesis behind. The fix used to leave one, which does not parse, so
the fix was rejected.
