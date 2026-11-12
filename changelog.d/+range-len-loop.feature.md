[`range-len-loop`](../src/rules/logic/range_len_loop.md) reports `for i in range(len(seq))` loops that read `seq[i]`,
and its unsafe fix loops over `seq` itself when the index serves only to read the items.
