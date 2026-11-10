class Base:
    def run(self, name: str) -> str:
        return name

    def stop(self, name):
        return name

    def store(self, name, *args, flag, **kwargs):
        return name


class Child(Base):
    def run(self, name):
        return super().run(name)

    def stop(self, name):
        """Stop the run."""
        return super().stop(name)

    def store(self, name, *args, flag, **kwargs):
        return super().store(name, *args, flag=flag, **kwargs)

    def typed(self, name: str) -> str:
        return super().run(name)


class Commented(Base):
    # Keeps the type checker from reporting the parent's iterator.
    def run(self, name):
        return super().run(name)


class Annotated(Base):
    """Forwarders with annotations."""

    def run(self, name: str) -> str:
        return super().run(name)


class Aliased(Base):
    def run(self, name):
        return make(name)

    def label(self, name):
        return name

    def variadic(*args):
        return super().variadic(*args)


class Named(Base):
    def run(self, name):
        return super().run(name)

    run: int

    class run:
        pass


class Defaults(Base):
    def run(self, name="x"):
        return super().run(name)


class Renamed(Base):
    def run(self, name):
        return super().run(name.strip())


class Discarded(Base):
    def run(self, name):
        super().run(name)


class Reordered(Base):
    def run(self, name, flag):
        return super().run(flag, name)


class Shadowed(Base):
    def run(self, name):
        return super().run(name)

    run = None


class Decorated(Base):
    @staticmethod
    def run(name):
        return super().run(name)


class Parent:
    def wide(self, name, extra=None):
        return name

    def named(self, other):
        return other

    @abstractmethod
    def abstract(self, name):
        return name

    async def waits(self):
        return None

    def keyed(self, *, flag):
        return flag

    def packed(self, *args):
        return args

    def mapped(self, **kwargs):
        return kwargs

    def ordered(self, name, /):
        return name

    def __eq__(self, other):
        return True

    __hash__ = object.__hash__

    twice = None
    twice = None


class Narrowed(Parent):
    def wide(self, name):
        return super().wide(name)

    def named(self, name):
        return super().named(name)

    def abstract(self, name):
        return super().abstract(name)

    def waits(self):
        return super().waits()

    def keyed(self, *, other):
        return super().keyed(other=other)

    def packed(self):
        return super().packed()

    def mapped(self):
        return super().mapped()

    def ordered(self, name):
        return super().ordered(name)

    def __eq__(self, other):
        return super().__eq__(other)

    def __hash__(self):
        return super().__hash__()

    def twice(self):
        return super().twice()

    def missing(self):
        return super().missing()


class Middle(Parent):
    pass


class Leaf(Middle):
    def ordered(self, name, /):
        return super().ordered(name)


class Imported(Mapping):
    def get(self, key):
        return super().get(key)


class Mixed(Parent, Mapping):
    def ordered(self, name, /):
        return super().ordered(name)


class Meta(Parent, metaclass=ABCMeta):
    def ordered(self, name, /):
        return super().ordered(name)


class Loop(Loop):
    def run(self, name):
        return super().run(name)


@decorate
class Rebuilt(Parent):
    pass


class FromRebuilt(Rebuilt):
    def ordered(self, name, /):
        return super().ordered(name)


def build():
    class Local(Parent):
        def ordered(self, name, /):
            return super().ordered(name)

    return Local


class Trailing(Base):
    def run(self, name):
        return super().run(name)  # keep: the base class validates the name
