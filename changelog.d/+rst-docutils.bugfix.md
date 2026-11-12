The docstring rules read reStructuredText the way docutils does.
[`rst-field-syntax`](../src/rules/docstrings/rst_field_syntax.md) treats a field after a bullet item, literal block or
directive line as a new field list, and reports only the first field joined to a paragraph.
[`rst-inline-markup`](../src/rules/docstrings/rst_inline_markup.md) follows docutils' rules for where inline markup
starts and ends: a literal closed by a pair followed by a letter is unclosed, backticks after a quote mark or inside
interpreted text are not, and comments, `raw` and `math` content, literal blocks, targets and substitution definitions
are not scanned. Its messages name the cause: a Markdown fence, a literal followed by a suffix, or backtick runs of
different lengths. [`rst-unknown-param`](../src/rules/docstrings/rst_unknown_param.md) and the other field rules read
escaped colons, colons in names and backticked names as docutils does.
