`unused-definition` (SMP409) and `test-only-code` (SMP408) keep a function that the documentation names with an autodoc
directive, so a test calling it no longer makes it test-only code.
