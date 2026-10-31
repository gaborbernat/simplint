`missing-from-all` and `undocumented-public-api` no longer treat a name reached only by a dotted string, such as a
`mocker.patch` target through a module's `import os`, as an export. Fixes that rename or move names still leave such
names alone.
