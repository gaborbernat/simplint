The docstring rules share the new `SMP3` group, so `select = ["SMP3"]` picks all of them: `function-docstring` is SMP300
(was SMP002), `test-module-docstring` SMP301 (was SMP004) and `undocumented-public-api` SMP302 (was SMP116); update
`select`, `ignore` and `noqa` comments that name the old codes. Docstrings no longer have a length limit:
`multiline-module-docstring` (SMP003) is gone, and `long-paragraph` (SMP005) checks comments only.
