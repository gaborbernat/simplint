Names that a `.pth` file reads at interpreter startup, such as `import pkg.hook; pkg.hook._run()`, now count as used
outside the tests, as entry points do. `test-only-code` (SMP408), `unused-export` (SMP402), `unexported-public-name`
(SMP403), `unused-definition` (SMP409) and `function-docstring` (SMP901) no longer report them as test-only, unused,
unlisted or undocumented.
