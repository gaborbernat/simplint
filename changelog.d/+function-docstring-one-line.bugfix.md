`function-docstring` (SMP002) and `test-module-docstring` (SMP004) fix a docstring that shares its line with the
signature or the next statement, as in `def run(): """Run."""`, instead of only after `ruff format` splits the line.
