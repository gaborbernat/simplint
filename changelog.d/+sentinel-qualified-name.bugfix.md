[`up-object-sentinel`](../src/rules/logic/up_object_sentinel.md) names a class attribute with its class, as
`sentinel("Config.UNSET")`, so pickling can find it, and no longer offers a fix for a placeholder in a class inside a
function.
