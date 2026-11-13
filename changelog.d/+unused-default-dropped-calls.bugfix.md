[`unused-default`](../src/rules/logic/unused_default.md) counts calls whose result is dropped when it matches a call to
a function the import resolver cannot place, such as a script on pytest's `pythonpath`. It removed aiobotocore's
`full_history` default that such a call still passed.
