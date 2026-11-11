class A:
    def run(self):
        return 1

    match 1:
        case run:
            pass

    def __init__(self):
        self.x = 1


class B:
    def run(self):
        return 1

    try:
        raise KeyError
    except KeyError as run:
        pass

    def __init__(self):
        self.x = 1


class C:
    def run(self):
        return 1

    match 1:
        case other:
            pass

    def __init__(self):
        self.x = 1


class D:
    def run(self):
        return 1

    match [1]:
        case [*run]:
            pass

    def __init__(self):
        self.x = 1


class E:
    def run(self):
        return 1

    match {}:
        case {**run}:
            pass

    def __init__(self):
        self.x = 1
