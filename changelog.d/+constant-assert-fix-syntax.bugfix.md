`constant-assert` (SMP804) no longer offers a fix that writes invalid syntax when the tuple holds a walrus or a starred
element: a walrus check gets parentheses, and a tuple with a starred element gets no fix.
