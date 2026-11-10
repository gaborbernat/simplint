[`single-user-constant`](../src/rules/layout/single_user_constant.md) inlines an integer constant as `(30).bit_length()`
when an attribute follows it. The fix used to write `30.bit_length()`, which does not parse, so the fix was rejected.
