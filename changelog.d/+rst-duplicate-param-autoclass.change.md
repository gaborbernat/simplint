[`rst-duplicate-param`](../src/rules/docstrings/rst_duplicate_param.md) follows the docstring Sphinx renders for each
class, resolved per project from a `:class-doc-from:` option on its `autoclass` or `autoexception` directive, then
`autodoc_default_options` for a class such a directive names, then `autoclass_content` in the project's Sphinx
configuration. A directive names a class by its dotted name, or by a simple name that the page's `module` or
`currentmodule` qualifies. With no module on the page, a simple name shared by two classes of the run leaves both alone.
Under `init` the rule reports the copy in the class docstring and removes that one, and under `both` it keeps reporting
the `__init__` copy. A configuration counts when it sits beside an index page or names `extensions`, `master_doc` or
`root_doc`, and a wildcard import does not count. A value the rule cannot read leaves every copy alone.
