[`up-compression-modules`](../src/rules/logic/up_compression_modules.md) reads the attributes of the file once and
checks each codec and name once, instead of walking the whole module for each import. A module of 32,000 functions that
each import `zlib` takes 0.11 seconds instead of 18.0.
