[`missing-override`](../src/rules/classes/missing_override.md) reports a method that defines again a method of a base
class in the run, across files, without `@typing.override`, on Python 3.12 and later. With the decorator a type checker
reports the method once the base renames or removes what it overrides. The safe fix adds `@override` next to the `def`
and imports it from `typing`.
