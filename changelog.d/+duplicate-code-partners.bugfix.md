[`duplicate-code`](../src/rules/logic/duplicate_code.md) (SMP530) names the first three other modules in sorted order,
one place each, and counts the rest. A module that many others repeat a block with no longer hides the modules after the
first ones behind a single module's copies, so the report names a module at the line where its copy starts.
