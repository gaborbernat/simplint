With `--fix`, a fix that changes what other files learn from the project index, such as `mock-attribute-assignment`
(SMP203) folding `user.uuid = 7` into the mock constructor, now gets the files depending on it linted and fixed again in
the same run, so `dataclass-options` (SMP111) freezes the dataclass whose `uuid` field the fix stopped assigning without
a second run.
