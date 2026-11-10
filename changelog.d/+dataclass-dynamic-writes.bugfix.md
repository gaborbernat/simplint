[dataclass-frozen](../src/rules/classes/dataclass_frozen.md) and
[dataclass-slots](../src/rules/classes/dataclass_slots.md) no longer offer a fix that breaks code setting attributes
under a computed name, through `monkeypatch.setattr` or `mock.patch.object`, weakly referencing the instance, or
decorating the class from below `@dataclass`. They also see writes to instances of private classes, such as `_Result`,
and of all-caps classes, such as `URL`.
