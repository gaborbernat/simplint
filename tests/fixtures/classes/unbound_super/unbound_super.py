__all__ = []


def outside():
    return super().method()


class Base:
    def method(self):
        return 1

    def items(self):
        return [1]


class Child(Base):
    attribute = super()

    def method(self):
        return super().method()

    def in_generator(self):
        return list(super().method() for _ in range(3))

    def in_first_iterable(self):
        return list(item for item in super().items())

    def in_comprehension(self):
        return [super().method() for _ in range(3)]

    def in_lambda(self):
        return (lambda: super().method())()

    def in_lambda_with_parameter(self):
        return (lambda value: super().method())(1)

    def in_nested(self):
        def inner():
            return super().method()

        return inner()

    def in_nested_with_parameter(self):
        def inner(value):
            return super().method()

        return inner(1)

    def explicit(self):
        return list(super(Child, self).method() for _ in range(3))
