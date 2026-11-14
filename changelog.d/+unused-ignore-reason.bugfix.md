The [`unused-ignore`](../src/rules/comments/unused_ignore.md) fix removes a stale directive's reason comment with it,
instead of leaving the reason behind as a bare comment. Another tool's directive later on the line, such as
`# noqa: E501`, stays.
