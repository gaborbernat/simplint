simplint stops with exit status 2 before linting when a simplint rule and a Ruff rule on for the same file work against
each other: `function-docstring` with Ruff's missing-docstring rules D101 to D107, `test-module-docstring` with D100 or
D104 on test modules, `future-annotations` with I002 adding `from __future__ import annotations` on Python 3.14 and
newer, and `absolute-sibling-import` with TID252 banning every relative import. The error names both rules, the
configuration files that enable them, the first file affected and how to resolve the clash. Before, the fix loop kept
the report forever, or gave up after 100 passes.
