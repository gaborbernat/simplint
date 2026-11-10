[rst-unknown-param](../src/rules/docstrings/rst_unknown_param.md) deletes a field from a docstring whose remaining text
ends in a quote or a backslash, and puts the closing quotes on a line of their own. The fix used to join them to that
text, which ends or escapes the string, so the fix was rejected.
