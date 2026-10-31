Module names in imports, attribute access, patch targets and entry points now resolve to files with ty's module
resolver, the one behind `ruff analyze graph`, searching Ruff's `src` directories and package roots. Cross-file rules
such as `missing-from-all` and the rename fixes see `src` layouts and pytest rootdir test packages correctly and no
longer match a module by a guessed dotted suffix of its path.
