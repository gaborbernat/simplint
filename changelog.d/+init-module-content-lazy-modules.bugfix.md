`init-module-content` (SMP407) accepts `__lazy_modules__` in a public package's `__init__.py`, which Python 3.15 reads
from the module globals to make the package's imports lazy.
