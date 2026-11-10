[walrus-candidate](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP502.html) looks up the other mentions
of each name in one index of the module instead of walking every statement before it, so its time no longer grows with
the square of a module's assignments: 16,000 assignment and `if` pairs take 0.06 seconds instead of 9.9.
