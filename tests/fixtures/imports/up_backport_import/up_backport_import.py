import tomli
import tomli as toml
from tomli import loads
from tomli import loads as parse
from exceptiongroup import BaseExceptionGroup, ExceptionGroup
from exceptiongroup import catch
from async_timeout import timeout
from taskgroup import TaskGroup
from importlib_metadata import version
from importlib_resources import files
from .tomli import loads as relative
import json
import exceptiongroup

try:
    import tomli as fallback
except ImportError:
    import tomllib as fallback


def run():
    from tomli import loads as inner

    return inner


print(tomli.loads("a = 1"), toml.loads("b = 2"), loads("c = 3"), parse, relative, json.dumps(1))
print(ExceptionGroup("x", [ValueError()]), BaseExceptionGroup, catch, timeout, TaskGroup, version, files)
print(fallback, run)
