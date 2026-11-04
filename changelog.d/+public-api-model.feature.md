`undocumented-public-api` (SMP302) and `function-docstring` (SMP300) now follow a library's public API as the typing
spec and PEP 8 define it: modules under a `_`-prefixed path are internal, a public module without `__all__` offers every
name without a leading underscore, and a name a public module re-exports through `__all__`, `from m import name as name`
or `import *` needs its docstring where it is defined. A name another module imports counts as public only in a public
module, where `missing-from-all` (SMP113) lists it in `__all__`, and an application, which nothing installs, has no
public API at all. The new `project-kind`, `api-packages` and `api-modules` settings tune this.
