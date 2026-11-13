[`unused-default`](../src/rules/logic/unused_default.md),
[`default-argument-value`](../src/rules/logic/default_argument_value.md),
[`unused-return-value`](../src/rules/api/unused_return_value.md) and
[`test-only-parameter`](../src/rules/api/test_only_parameter.md) now judge a function under a decorator that passes each
call on unchanged, such as `functools.cache`, `contextlib.contextmanager` or `typing.override`, instead of skipping it.
