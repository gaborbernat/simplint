`collection-built-by-statements` (SMP531) reports an empty dict, list or set that the statements right after it only
fill with entries, and writes those entries in the literal. The fix is safe when every key and value is a constant, and
unsafe when one is a name, since a name that is not defined would leave the collection partly filled.
