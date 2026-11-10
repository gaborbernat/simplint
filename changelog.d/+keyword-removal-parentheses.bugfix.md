[up-argparse-suggest-on-error](../src/rules/logic/up_argparse_suggest_on_error.md) and
[up-utf8-encoding-default](../src/rules/logic/up_utf8_encoding_default.md) keep the parentheses around a positional
argument that precedes a removed keyword, so `ArgumentParser(("prog"), suggest_on_error=True)` no longer becomes invalid
code.
