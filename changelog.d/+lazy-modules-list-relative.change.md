[`up-lazy-modules-list`](../src/rules/imports/up_lazy_modules_list.md) (SMP307) reads `f"{__spec__.parent}.name"`
entries and matches relative imports against the list, so it converts them to `lazy` imports too.
