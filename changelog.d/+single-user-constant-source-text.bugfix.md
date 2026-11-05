`single-user-constant` (SMP202) no longer offers a fix for a constant read in a t-string, where inlining or renaming
would change the kept text. The constant is still reported. A constant read in an f-string `=` debug field is inlined or
renamed with the printed text unchanged.
