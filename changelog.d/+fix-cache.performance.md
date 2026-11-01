With `--fix` or `--diff`, the cache also remembers files the fixes leave as they are, together with the rules whose
fixes simplint held back for them, so a later run reports such a file from a single lint instead of running the whole
fix loop again. On an already fixed copy of `sympy`, a repeat `--fix --unsafe-fixes` run takes about a quarter of the
CPU time it did.
