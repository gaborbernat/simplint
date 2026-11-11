simplint no longer reports what a Ruff rule reports, preview rules included, whether or not the project's Ruff
configuration selects that rule; before, a rule left a case to Ruff only while Ruff ran the rule. Rules that already
left cases to Ruff now always do, and these rules leave Ruff's cases for the first time:
[`private-import-path`](../src/rules/imports/private_import_path.md) (PLC2701),
[`init-module-content`](../src/rules/api/init_module_content.md) (F401),
[`up-backport-import`](../src/rules/imports/up_backport_import.md) (A004, when it has no fix),
[`stdlib-mock`](../src/rules/testing/stdlib_mock.md) (TID251, when `banned-api` bans the import),
[`section-comment`](../src/rules/comments/section_comment.md) (E265 and E266, for a heading written as prose),
[`function-docstring`](../src/rules/docstrings/function_docstring.md) and
[`test-module-docstring`](../src/rules/docstrings/test_module_docstring.md) (D418 and D419, when they have no fix) and
[`unused-default`](../src/rules/logic/unused_default.md) (PT028, when it has no fix). Select the Ruff rules to have
these cases reported. [`undocumented-public-api`](../src/rules/docstrings/undocumented_public_api.md) still leaves a
case to Ruff's D1 rules only while they run, since the recommended setup turns them off.
