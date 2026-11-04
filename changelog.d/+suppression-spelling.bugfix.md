`suppression-without-reason` (SMP060) reads suppression comments the way they take effect, so it now also flags
`# flake8: noqa` and a directive with extra space around its colon, such as `# ruff :  ignore[...]`.
