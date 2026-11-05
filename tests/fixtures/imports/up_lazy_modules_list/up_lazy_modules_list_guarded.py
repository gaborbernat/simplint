__lazy_modules__ = ["json"]

try:
    import json
except ImportError:
    json = None
