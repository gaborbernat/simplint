[`flattened-package`](../src/rules/imports/flattened_package.md) reports each private module whose name shares a
`_<prefix>_` prefix with two or more siblings in its directory, such as `_html_url.py`, `_html_parse.py` and
`_html_css.py`, which a `_html` package that re-exports their names would group. It checks a module again when a sibling
joins or leaves its directory.
