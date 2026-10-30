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
