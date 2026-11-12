[`async-sync-dunder`](../src/rules/classes/async_sync_dunder.md) reports `async def` on special methods Python calls
without awaiting, such as `__len__`, `__enter__` and a non-generator `__aiter__`.
