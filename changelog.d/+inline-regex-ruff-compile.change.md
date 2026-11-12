[`inline-regex`](../src/rules/logic/inline_regex.md) no longer stops the run as a conflict with Ruff's
`unnecessary-regular-expression-compile`, so both can stay on. Ruff reports a method called on an inline `re.compile`
and has no fix. This rule's fix stores the pattern in a module-level constant, which neither rule reports.
