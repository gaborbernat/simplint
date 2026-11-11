[`import-cycle`](../src/rules/imports/import_cycle.md) finds the cycle of a ring of modules without a search from each
module and keeps only the shown files of each long cycle in memory: a ring of 32,000 modules takes 2.1 seconds and 0.5
GB instead of 50.9 seconds and 4.8 GB.
