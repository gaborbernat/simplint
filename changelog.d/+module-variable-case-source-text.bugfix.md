`module-variable-case` (SMP102) no longer renames a name read in a t-string, where the kept text would change. The name
is still reported, without a fix. A name read in an f-string `=` debug field is renamed with the printed text unchanged.
