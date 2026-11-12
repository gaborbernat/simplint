[`exception-init-args`](../src/rules/classes/exception_init_args.md) reports exception classes whose `__init__`
passes its parent other positional arguments than it takes, or requires a keyword-only one, so `pickle` and `copy`
cannot rebuild them.
