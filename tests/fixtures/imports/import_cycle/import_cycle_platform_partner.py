import sys

if sys.version_info < (3, 99):
    if sys.version_info < (3, 50):
        pass
    if sys.platform == "linux":
        import import_cycle_platform

VALUE = 1
