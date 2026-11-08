`--fix --unsafe-fixes` no longer stops with "Failed to converge after 100 iterations" and applies nothing when a file
has about 100 identical fixes, such as `single-use-variable` inlines of `lock = build()`, and one of them is withdrawn.
The withdrawn fix stays withheld while the others apply.
