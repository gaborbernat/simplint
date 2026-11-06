`up-warnings-deprecated` (SMP524) reports functions and classes that start with a `DeprecationWarning` warning from
Python 3.13, where `@warnings.deprecated` marks them for type checkers. The fix is unsafe: a class then warns when
instantiated, not when defined.
