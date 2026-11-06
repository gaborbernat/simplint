import sys

if sys.version_info < (3, 99):
    pass
elif sys.version_info[1] < 3:
    pass
elif sys.version_info != (3, 14):
    pass
elif sys.version_info < (3, 14, 0):
    pass
elif sys.version_info[0] < (3,):
    pass
elif sys.platform.major < 3:
    pass
elif 0 < sys.version_info[0] < 3:
    pass
elif sys.platform == "win32":
    import import_cycle_platform_partner
