Ruff fixes that depend on the line length, such as collapsing nested `if` statements, are judged the same before and
after `ruff format`, so a second run no longer applies what the first withdrew.
