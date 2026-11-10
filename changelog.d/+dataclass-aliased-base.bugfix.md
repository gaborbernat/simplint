[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) and
[`dataclass-kw-only`](../src/rules/classes/dataclass_kw_only.md) no longer offer a fix for a class whose subclass
imports it under another name, as `from models import Release as Base` followed by `class Tagged(Base)`.
