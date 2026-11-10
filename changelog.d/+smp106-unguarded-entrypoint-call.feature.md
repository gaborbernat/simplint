[`unguarded-entrypoint-call`](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP106.html) uses script
metadata, shebangs, executable bits, and `__main__.py` to report trailing entrypoint calls regardless of their name.
Single-line statements have an unsafe guard fix.
