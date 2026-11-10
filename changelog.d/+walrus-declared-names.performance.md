[`walrus-candidate`](../src/rules/logic/walrus_candidate.md) reads the names a function declares `global` or `nonlocal`
once per function instead of once per assignment it checks, so 32,000 assignment and `if` pairs in one function take
0.45 seconds instead of 4.7.
