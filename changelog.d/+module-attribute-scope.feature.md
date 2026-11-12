[`module-attribute-import`](../src/rules/imports/module_attribute_import.md) takes an `imports.module-attribute-scope`
setting that widens it from the project's own modules to the standard library (`"stdlib"`), every other module
(`"third-party"`) or one module and its submodules by name, such as `"requests"`. The default, `["project"]`, keeps
today's behavior. A module outside the run is reported without a fix. A module imported under the alias Ruff's
`flake8-import-conventions` asks for, such as `numpy` as `np`, one its `banned-from` lists, and a name a string patches,
such as `mocker.patch("requests.get")`, stay reached through the module.
