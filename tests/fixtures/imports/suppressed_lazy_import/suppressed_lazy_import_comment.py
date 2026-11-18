from contextlib import suppress

__lazy_modules__ = [
    # optional speedups
    "ujson",
]

with suppress(ImportError):
    import ujson
