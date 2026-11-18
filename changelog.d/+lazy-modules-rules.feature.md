Four rules check lazy imports ([PEP 810](https://peps.python.org/pep-0810/)):
[`missing-lazy-module`](../src/rules/imports/missing_lazy_module.md) (SMP314) lists in `__lazy_modules__` an import
nothing uses while the module loads, [`unsorted-lazy-modules`](../src/rules/imports/unsorted_lazy_modules.md) (SMP315)
sorts the list, [`ineffective-lazy-declaration`](../src/rules/imports/ineffective_lazy_declaration.md) (SMP316) reports
entries and `lazy` imports that defer nothing, and
[`suppressed-lazy-import`](../src/rules/imports/suppressed_lazy_import.md) (SMP317) reports a lazy import inside
`suppress(ImportError)`. The `imports.require-lazy-modules` setting makes SMP314 check modules without a list.
