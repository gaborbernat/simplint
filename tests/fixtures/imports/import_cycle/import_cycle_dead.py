import sys

_false = 0
_off: bool = False
_changed = 0
_changed += 1

if False:
    import import_cycle_dead_partner
if 0:
    import import_cycle_dead_partner
elif None:
    import import_cycle_dead_partner
if _false:
    import import_cycle_dead_partner
if _off:
    import import_cycle_dead_partner
if True:
    pass
else:
    import import_cycle_dead_partner
if sys.version_info < (3, 8):
    import import_cycle_dead_partner
if sys.version_info <= (3,):
    import import_cycle_dead_partner
if sys.version_info >= (3, 8):
    pass
else:
    import import_cycle_dead_partner
if sys.version_info > (3, 9):
    pass
elif sys.version_info[0] == 2:
    import import_cycle_dead_partner
if sys.version_info.major < 3:
    import import_cycle_dead_partner
if sys.version_info[0] <= 2:
    import import_cycle_dead_partner
if sys.version_info.major > 2:
    pass
else:
    import import_cycle_dead_partner
try:
    from _import_cycle_speedups import fast
except ImportError:
    from import_cycle_dead_partner import fast
try:
    from _import_cycle_speedups import slow
except (ModuleNotFoundError, AttributeError):
    from import_cycle_dead_partner import slow

if _changed:
    import import_cycle_dead_partner
