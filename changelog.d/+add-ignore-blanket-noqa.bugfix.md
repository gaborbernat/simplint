`--add-ignore` writes its `# simplint: ignore[...]` comment on a line with a blanket `# noqa` too, so the
[`blanket-ignore`](../src/rules/comments/blanket_ignore.md) and
[`suppression-without-reason`](../src/rules/comments/suppression_without_reason.md) reports on that line stop showing.
