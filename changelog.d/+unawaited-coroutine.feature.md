[`unawaited-coroutine`](../src/rules/logic/unawaited_coroutine.md) reports a call of an `async def` as a statement of
its own, whose body never runs, and adds the missing `await` inside a coroutine.
