import sys

if sys.platform == "win32":

    def open_thing() -> int: ...

else:

    def open_thing() -> int: ...


try:
    from _accel import fast
except ImportError:

    def fast(data: bytes) -> int:
        """Hash the data in pure Python, slower than the C version."""
        return len(data)
