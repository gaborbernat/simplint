[up-comprehension-unpacking](../src/rules/logic/up_comprehension_unpacking.md),
[up-traceback-exception-arguments](../src/rules/logic/up_traceback_exception_arguments.md),
[up-exc-info-exception](../src/rules/logic/up_exc_info_exception.md) and
[up-strptime-date](../src/rules/logic/up_strptime_date.md) no longer delete a comment in the code they rewrite. They
report the code without a fix when a comment lies in it.
