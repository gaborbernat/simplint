[`unformatted-exception-args`](../src/rules/logic/unformatted_exception_args.md) reports a builtin exception raised with
a `%` or `{}` message and the values meant to fill it, such as `raise ValueError("bad %s", value)`, which shows a tuple
instead of the message. Its unsafe fix formats the message.
