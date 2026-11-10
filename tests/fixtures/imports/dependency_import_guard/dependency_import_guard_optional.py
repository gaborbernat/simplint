try:
    import _overlapped
except ImportError:
    _overlapped = None

try:
    import _gdbm
except ImportError:
    _gdbm = None

try:
    import asyncio.windows_events
except ImportError:
    asyncio = None

try:
    import mmap
except ImportError:
    mmap = None

try:
    import gzip
except ImportError:
    gzip = None

try:
    import _testcapi
except ImportError:
    _testcapi = None
