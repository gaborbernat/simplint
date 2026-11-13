import builtins

try:
    import tomllib
except ImportError:
    import tomli as tomllib

try:
    import requests
except ModuleNotFoundError:
    requests = None

try:
    import optional_extra
except ImportError:
    optional_extra = None

try:
    import json
except (ImportError, AttributeError):
    json = None
else:
    pass

try:
    import fcntl
except ImportError:
    fcntl = None

try:
    from json import loads
except ImportError:
    loads = None

try:
    from . import sibling
except ImportError:
    sibling = None

try:
    import requests
    FLAG = True
except ImportError:
    FLAG = False

try:
    import requests
except builtins.ImportError:
    requests = None

try:
    from distutils import dist
except ImportError:
    dist = None

try:
    import ctypes
except ImportError:
    ctypes = None

try: import tomllib
except ImportError: import tomli as tomllib

try: import json; import tomllib  # both required
except ImportError:
    json = None

try:
    import asyncio.taskgroups
except ImportError:
    asyncio = None

try:
    import _zstd
except ImportError:
    _zstd = None

try:
    import _curses
except ImportError:
    _curses = None

try:
    import os.path
except ImportError:
    os = None

try:
    import string.templatelib
except ImportError:
    pass

try:
    import compression.zstd
except ImportError:
    pass

try:
    import os.nonexistent
except ImportError:
    pass

try: import json; import tomllib
except ImportError:
    json = None
print(json)

try:
    import zoneinfo as zones
except ImportError:
    pass

try:
    import requests
except builtins.ModuleNotFoundError:
    requests = None

try:
    import _socket
    import tracemalloc
except ImportError:
    _socket = tracemalloc = None
print(_socket, tracemalloc)
