`single-letter-name` (SMP010) now reports `for` loop targets such as `i` in `for i in range(3)`, since a loop body can run
far from the line that says what the name holds. Names inside comprehensions stay exempt.
