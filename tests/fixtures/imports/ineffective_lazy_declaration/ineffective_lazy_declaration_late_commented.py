import json

# Deferred modules.
__lazy_modules__ = ["json"]


def use() -> object:
    return json
