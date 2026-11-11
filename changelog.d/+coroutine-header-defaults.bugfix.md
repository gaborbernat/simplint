[`up-event-loop-in-coroutine`](../src/rules/logic/up_event_loop_in_coroutine.md) no longer rewrites a decorator, default
or annotation of an `async def`, which runs outside any loop, so `async def serve(loop=asyncio.get_running_loop())` no
longer raises at import. The same defaults of a nested `def` inside a coroutine are now reported, and so are its
annotations unless annotations are lazy.
