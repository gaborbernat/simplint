[`unused-definition`](../src/rules/api/unused_definition.md) treats a class that defines `__getnewargs__` or
`__getnewargs_ex__` as read whole, as it already did for `__reduce__`: pickle rebuilds such objects through those hooks.
