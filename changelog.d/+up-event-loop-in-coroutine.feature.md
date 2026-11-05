`up-event-loop-in-coroutine` (SMP515) flags `asyncio.get_event_loop()` calls in coroutines, which
`asyncio.get_running_loop()` replaces on Python 3.10 and later, with a safe fix.
