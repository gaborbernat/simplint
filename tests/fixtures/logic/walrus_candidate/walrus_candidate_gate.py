import sys

if sys.version_info < (3, 6):
    sys.exit("unsupported")


def first(items):
    head = items[0]
    if head:
        return head
    return None
