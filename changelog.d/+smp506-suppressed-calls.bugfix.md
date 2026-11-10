[`unused-default`](../src/rules/logic/unused_default.md) counts a positional call that suppresses
[`dataclass-kw-only`](../src/rules/classes/dataclass_kw_only.md) like any other call, instead of treating it as a use it
cannot follow and keeping the function's defaults.
