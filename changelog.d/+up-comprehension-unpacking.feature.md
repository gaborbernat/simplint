`up-comprehension-unpacking` (SMP508) flags a nested `for` clause that only unpacks its iterable, which Python 3.15 lets
the element do with `*` or `**`, with a safe fix for lists, sets and generators.
