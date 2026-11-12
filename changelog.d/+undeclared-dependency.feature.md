[`undeclared-dependency`](../src/rules/imports/undeclared_dependency.md) reports an import of a third-party module
that no requirement or extra in `[project]` installs, in the modules that run when the package is imported. Such an
import works where a development tool or another package installed the module and fails for a user who installs only
what the project lists. Plugin modules, tests, and guarded, deferred or version-dependent imports are left alone.
