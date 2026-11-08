`absolute-sibling-import` (SMP302) no longer reports or fixes a project that runs flake8-lazy, configured by a
`[tool.flake8-lazy]` table or a pre-commit or prek hook with id `flake8-lazy`. Its fix made flake8-lazy rewrite each
module's `__lazy_modules__` list into entries that `ty` rejects.
