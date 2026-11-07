`up-unittest-assert-helper` (SMP811) stops with a conflict error when the project's Ruff configuration selects
[PT009](https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/), since both report the same `self.assertTrue` and
`self.assertFalse` calls and fix them in opposite ways.
