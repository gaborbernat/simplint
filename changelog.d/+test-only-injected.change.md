The [`test-only-parameter`](../src/rules/api/test_only_parameter.md) rule no longer reports a defaulted parameter that
injects a collaborator, such as a class-typed parameter the body uses, a `None` default the body calls, or a default
that resolves to a function, a class, a module or an instance built by calling a class, since a test passes a stand-in
for it by design. Data, names the rule cannot resolve, and enums stay reported, so a `None` default the body only reads
is reported too.
