[`early-assignment`](../src/rules/logic/early_assignment.md) searches the scopes nested in a function by offset instead
of testing each mention against all of them. A function of 80,000 lambdas takes 0.10 seconds under SMP526 instead of
2.3.
