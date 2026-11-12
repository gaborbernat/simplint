[`rst-field-syntax`](../src/rules/docstrings/rst_field_syntax.md) reads field lists the way docutils does. A field after
a bullet item, a literal block or a directive line starts a field list of its own, so it is no longer reported as joined
to text. A field inside a quoted literal block is code, not a field. Of the fields that follow a paragraph, only the
first is reported.

[`rst-inline-markup`](../src/rules/docstrings/rst_inline_markup.md) applies docutils' rules for where inline markup
starts and ends. An inline literal whose closing backticks docutils does not accept, such as a closing pair followed by
a letter, is now reported as unclosed. A pair of backticks after a quote mark, or inside interpreted text, is no longer
reported. Comments, `raw` and `math` content, quoted literal blocks, hyperlink targets and substitution definitions are
not scanned for roles or literals. A bullet or enumerated item's body is measured from where its text starts, so a
literal block under `- Example::` is code. An enumerator without a valid ordinal, or without an indented or
next-enumerator line after it, is text, and a line followed by an underline is a section title whose text is read as
inline markup.

A Markdown code fence in a reST docstring is reported once, at its opening line, with the advice to use `::` or
`.. code-block::`. A literal whose closing pair is followed by a letter or digit names the literal and the suffix to
write after a backslash. A backtick run closed by a run of the other length is reported as mismatched backticks, even
where docutils reads the pair as markup, since the output renders wrong.

[`rst-unknown-param`](../src/rules/docstrings/rst_unknown_param.md) and the other field rules read field names with
escaped colons, colons inside the name, and backticks around the name the way docutils does. They compare the name
without its backticks.
