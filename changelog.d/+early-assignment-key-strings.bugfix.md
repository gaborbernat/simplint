[`early-assignment`](../src/rules/logic/early_assignment.md) no longer holds an assignment back for a string that keys a
subscript or a dict display and spells the name, such as `loaded["cases"]`, since such a string is never run as code.
