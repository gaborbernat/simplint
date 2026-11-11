[`up-asyncio-iscoroutinefunction`](../src/rules/logic/up_asyncio_iscoroutinefunction.md) looks up the imports of the
module in a set built once per file, instead of scanning the module body for each call. A module of 32,000
`asyncio.iscoroutinefunction()` calls takes 0.10 seconds instead of 1.5.
