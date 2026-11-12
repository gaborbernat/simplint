The [`parametrize-without-id`](../src/rules/testing/parametrize_without_id.md) rule now reports an `ids=` list of string
literals, since the list drifts from the rows it names, and offers a fix that moves each id into its row as
`pytest.param(..., id=...)`. A callable or a name passed as `ids=` still generates the ids and is left alone.
