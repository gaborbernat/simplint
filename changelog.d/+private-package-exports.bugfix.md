`private-import-path` now accepts names exported by a direct private child of the top-level package, such as
`from pkg._core import Settings`, within the package and its distribution's tests.
