SIZE = 3


class Base:
    def describe(self):
        return "base"


class AnnotatedParent:
    describe: int


class ChildOfAnnotated(AnnotatedParent):
    def describe(self):
        return super().describe()


class NestedParent:
    class describe:
        pass


class ChildOfNested(NestedParent):
    def describe(self):
        return super().describe()


class DecoratorRead(Base):
    def describe(self):
        return super().describe()

    @register(describe)
    def label(self):
        return "label"


class ClassDecoratorRead(Base):
    def describe(self):
        return super().describe()

    @register(describe)
    class Inner:
        pass


class BaseRead(Base):
    def describe(self):
        return super().describe()

    class Inner(describe):
        pass


class LambdaDefaultRead(Base):
    def describe(self):
        return super().describe()

    label = lambda value=describe: value


class ExceptBound(Base):
    def describe(self):
        return super().describe()

    try:
        import json
    except ImportError as describe:
        pass


class MatchBound(Base):
    def describe(self):
        return super().describe()

    match SIZE:
        case describe:
            pass


class MatchRests(Base):
    def describe(self):
        return super().describe()

    match SIZE:
        case [1, *describe]:
            pass
        case {**describe}:
            pass
        case 1:
            pass


class Plain(Base):
    def describe(self):
        return super().describe()
