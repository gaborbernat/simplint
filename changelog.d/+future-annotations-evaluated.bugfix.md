[future-annotations](../src/rules/imports/future_annotations.md) marks its fix unsafe when an annotation unions a string
with a type, such as `"Node" | None`, or when an import sits under a module-level `if`, since the annotations then fail
once evaluated. It no longer offers a fix that would delete a comment on the import's lines.
