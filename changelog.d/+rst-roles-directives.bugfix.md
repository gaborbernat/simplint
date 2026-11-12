The reStructuredText and MyST scanners behind the documentation rules now read roles and directives as docutils and MyST
do. `rst-inline-markup` no longer reports a bare role inside an inline literal, a comment or raw content, and now
reports one inside quotes or after interpreted text, which docutils reads as a role. `unused-definition` and the other
rules that read the project's pages count the roles and directives docutils reads: suffix roles, targets and explicit
titles wrapped across lines, directives after enumerators such as `1.`, `(a)` or `#.`, directives in grid and simple
table cells, directives in substitution definitions, wrapped options, `{eval-rst}` content, and MyST fences with three
or more markers, tilde fences and `---` option blocks. A `| text` line outside a grid table is a line block, so it holds
no directive. Indented code in MyST needs four spaces past the content of the list item it follows, lazy continuation
lines stay in their list item, and `* * *` or `---` are thematic breaks rather than list items. Roles next to non-ASCII
punctuation follow docutils' Unicode punctuation classes, which hold ranges of characters, so an em dash, an ellipsis,
`。` or `．` before or after a role counts as a delimiter, as do `«»`, `‘’` and CJK brackets. Each line of a directive's
argument is a signature, as Sphinx reads them, and a tab counts to the next multiple of eight columns in
reStructuredText and four in MyST. MyST fences on a list item's marker line or in a blockquote are read, and comments,
HTML blocks, autolinks and link destinations hold no roles. The rules no longer count roles in code, comments, emphasis
or escaped text, or directives in literal blocks and comments.
