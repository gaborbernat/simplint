[`forwarding-function`](../src/rules/logic/forwarding_function.md) (SMP501) and
[`single-caller-helper`](../src/rules/logic/single_caller_helper.md) (SMP505) now count a call whose `(` starts a later
line, as in `return [_fill_a` and `(name)]`, the same as a call on one line. A name that only ends its statement, such
as `callback = _make_path`, is still only read, not called.
