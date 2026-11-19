A change to a Python file invalidates its cached results even when the new contents have the same length, permissions
and modification time. Unchanged files on supported local filesystems can reuse results without rereading the source.
