# A `return` outside a function is a syntax error the parser keeps, and leaves no function to name.
class Misplaced:
    return 1, 2, 3, 4, 5, 6, 7, 8, 9
