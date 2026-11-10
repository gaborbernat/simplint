[unnamed-regex-group](../src/rules/logic/unnamed_regex_group.md) no longer makes the groups of a pattern non-capturing
when the code reads them by number through a class attribute, a name declared `global` or `nonlocal`, or a module name
that another module imports or `__all__` lists, reads the rule does not follow.
