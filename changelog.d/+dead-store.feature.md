[`dead-store`](../src/rules/logic/dead_store.md) reports a value assigned to a local that the next assignment in the
same block replaces before anything reads it, such as `total = 0` right before `total = len(items)`. Its fix removes the
assignment when the value is a constant.
