`single-user-constant` and `walrus-candidate` parenthesize an inlined value that starts with a parenthesis without being
wrapped in one, such as `(RuntimeError,) if old else ()`, keep a tuple's parentheses, and inline a value written over
several lines on one; `single-user-constant` no longer derives a local name that starts with a digit or is a keyword.
