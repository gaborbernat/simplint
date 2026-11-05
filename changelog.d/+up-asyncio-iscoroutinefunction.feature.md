`up-asyncio-iscoroutinefunction` (SMP516) flags `asyncio.iscoroutinefunction()` calls, which
`inspect.iscoroutinefunction()` replaces, with an unsafe fix since the two differ for legacy coroutine markers.
