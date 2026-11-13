__all__ = []


class Point:
    def __init__(self, x):
        self.x = x

    def __eq__(self, other):
        if not isinstance(other, Point):
            return False
        return self.x == other.x

    def __lt__(self, other):
        if type(other) is not Point:
            raise TypeError("cannot compare")
        return self.x < other.x

    def __add__(self, other):
        if type(other) != Point:
            raise TypeError
        return Point(self.x + other.x)

    def __sub__(self, other):
        if not isinstance(other, Point):
            # keep the old answer
            return False
        return Point(self.x - other.x)

    def __mul__(self, other):
        if not isinstance(other, Point):
            return NotImplemented
        return Point(self.x * other.x)

    def __hash__(self):
        return hash(self.x)

    def merge(self, other):
        if not isinstance(other, Point):
            return False
        return True

    def __or__(self, other):
        if not isinstance(other, Point):
            raise ValueError("no")
        return self


class Vector:
    def __eq__(self, other, /):
        if not isinstance(other, Vector):
            return False
        return True

    def __ne__(self, *others):
        return NotImplemented

    def __lt__(self, other):
        if not isinstance(other, Vector):
            print("rejected")
            return False
        return True

    def __le__(self, other):
        if not isinstance(other, Vector):
            return False
        elif other is self:
            return True
        return False

    def __gt__(self, other):
        if isinstance(other, int):
            return False
        return True
