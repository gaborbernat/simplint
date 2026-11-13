import asyncio
import time
from time import sleep


def test_sleep():
    time.sleep(1)


def test_from_import():
    sleep(0.5)


def test_zero():
    time.sleep(0)


def test_keyword_zero():
    time.sleep(secs=0)


async def test_async():
    await asyncio.sleep(2)


async def test_async_zero():
    await asyncio.sleep(0)


def test_other_patch(mocker):
    mocker.patch("os.getcwd")
    time.sleep(1)


import pytest  # noqa: E402


@pytest.fixture
def slow_server():
    time.sleep(1)
    yield


def serve_slowly():
    time.sleep(1)


class SlowDouble:
    def respond(self):
        time.sleep(1)


def test_with_callback():
    def callback():
        time.sleep(1)

    class Handler:
        def handle(self):
            time.sleep(1)

    callback()
    Handler().handle()
