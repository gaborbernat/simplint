"""Public package with lazily loaded submodules."""

__lazy_modules__ = ["lazy_modules._client"]
__lazy_modules__ += ["lazy_modules._extra"]

from lazy_modules._client import Client

__all__ = ["Client"]
VALUE = 1
