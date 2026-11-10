[`single-caller-helper`](../src/rules/logic/single_caller_helper.md) finds the caller of each helper in a map of the
file's functions instead of scanning them, and checks the size of a caller before walking it for loops, so 64,000
helpers take 0.31 seconds instead of 2.9 and a caller with 32,000 calls to helpers takes 0.14 seconds instead of 8.8.
