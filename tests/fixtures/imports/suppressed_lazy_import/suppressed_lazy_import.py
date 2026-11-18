import contextlib
from contextlib import suppress

__lazy_modules__ = ["ujson", "orjson", "pickle", "simplejson", "rapidjson", "yaml"]

import yaml

with suppress(ImportError):
    lazy import msgpack
    import ujson

with contextlib.suppress(KeyError, ModuleNotFoundError):
    from orjson import dumps

with suppress(Exception):
    import simplejson, rapidjson

with suppress(BaseException):
    import yaml

with suppress(ImportError):
    lazy from json import dumps as json_dumps, loads
    from pickle import dumps as pickle_dumps, loads as pickle_loads

with suppress(KeyError):
    lazy import cbor2

with suppress(ImportError), open("x"):
    lazy import toml

with open("x"):
    lazy import tomli

with CONTEXT:
    lazy import tomlkit

with suppress(ValueError, ImportError):
    pass


def use() -> object:
    return msgpack, ujson, dumps, simplejson, rapidjson, yaml, cbor2, toml, tomli, tomlkit, json_dumps, loads, pickle_dumps, pickle_loads
