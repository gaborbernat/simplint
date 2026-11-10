[up-re-error-alias](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP518.html) leaves `re.error = ...` and
`del re.error` alone, which patch the module rather than read the exception.
