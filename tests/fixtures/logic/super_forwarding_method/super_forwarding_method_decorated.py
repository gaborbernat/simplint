from dataclasses import dataclass


class Base:
    def run(self, name):
        return name


@dataclass
class Record(Base):
    def run(self, name):
        return super().run(name)


class Plain:
    def run(self, name):
        return super().run(name)


class Overridden(Base):
    @override
    def run(self, name):
        return super().run(name)
