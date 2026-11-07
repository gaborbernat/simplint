`single-use-variable` no longer nests an f-string or string literal inside the f-string it is inlined into, which it
wrote as `f"[{f'<{value}>'}]"`; the value's text joins the enclosing f-string instead, as `f"[<{value}>]"`.
