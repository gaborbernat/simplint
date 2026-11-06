`constant-assert` (SMP804) leaves a tuple test it cannot fix, such as `assert (1, 2, 3)`, to Ruff's F631 when the
project's Ruff configuration selects it. A two-element tuple without a message keeps its report and fix.
