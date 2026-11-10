[unused-default](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP506.html) counts a positional call that
suppresses [dataclass-kw-only](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP703.html) like any other
call, instead of treating it as a use it cannot follow and keeping the function's defaults.
