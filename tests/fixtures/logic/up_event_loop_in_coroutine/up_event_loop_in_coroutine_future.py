from __future__ import annotations

import asyncio


async def run():
    def later(loop: asyncio.get_event_loop()):
        return loop

    return later
