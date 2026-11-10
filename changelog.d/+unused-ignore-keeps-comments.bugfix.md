[unused-ignore](../src/rules/comments/unused_ignore.md) (SMP009) removes only the directive and its reason when the last
selector of a comment is unused. A comment after them on the line, such as `# noqa: E501`, stays instead of being
deleted with them.
