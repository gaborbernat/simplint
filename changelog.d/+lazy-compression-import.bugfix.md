[`up-compression-modules`](../src/rules/logic/up_compression_modules.md) keeps the `lazy` keyword of a
`lazy import gzip`, so the rewritten `lazy from compression import gzip` stays lazy instead of becoming an eager import.
