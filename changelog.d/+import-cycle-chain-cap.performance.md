[import-cycle](../src/rules/imports/import_cycle.md) keeps the modules of a cycle as ids and names only the first eight
of a long one, counting the rest, so a ring of 4,000 modules that import each other in turn takes 0.7 seconds and 172 MB
instead of 2.4 seconds and 3.3 GB.
