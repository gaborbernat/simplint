`long-tuple-return` (SMP528) reports a function that returns a tuple of more elements than the limit, which is two by
default, since a tuple of three or more values is a record whose fields a dataclass or a named tuple would name. The
limit is set with `max-tuple-elements` under `[tool.simplint.logic]`.
