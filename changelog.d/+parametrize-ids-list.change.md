The [`parametrize-without-id`](../src/rules/testing/parametrize_without_id.md) rule now reports an `ids=` list of string
literals, since the list drifts from the rows it names, and offers a fix that moves each id into its row as
`pytest.param(..., id=...)`. A module-level name bound once to such a list is reported without a fix. A callable, or a
name the rule cannot read, is left alone.
