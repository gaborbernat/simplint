The rules that ask whether `__all__` lists a name check a set of the listed names, built once per file, instead of
scanning the list for each name: [`missing-from-all`](../src/rules/api/missing_from_all.md) (SMP401),
[`unexported-public-name`](../src/rules/api/unexported_public_name.md) (SMP403),
[`test-only-code`](../src/rules/api/test_only_code.md) (SMP408),
[`unused-definition`](../src/rules/api/unused_definition.md) (SMP409),
[`test-only-parameter`](../src/rules/api/test_only_parameter.md) (SMP410) and
[`unused-return-value`](../src/rules/api/unused_return_value.md) (SMP411). A module of 64,000 functions, all listed in
`__all__`, takes 0.09 seconds under SMP401 instead of 2.8.
