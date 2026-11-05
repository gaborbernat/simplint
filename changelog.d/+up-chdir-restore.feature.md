`up-chdir-restore` (SMP513) flags a working directory restored in a `finally` clause, which `contextlib.chdir` does on
Python 3.11 and later, with a safe fix that adds the `contextlib` import.
