`up-exc-info-exception` (SMP510) flags `sys.exc_info()` reads of the exception being handled, which `sys.exception()`
returns directly on Python 3.11 and later, with a safe fix.
