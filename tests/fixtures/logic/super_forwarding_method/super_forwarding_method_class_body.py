import sys


class Base:
    def describe(self):
        return "base"


class ReadInClassBody(Base):
    def describe(self):
        return super().describe()

    label = describe


class BoundUnderIf(Base):
    def describe(self):
        return super().describe()

    if sys.version_info > (3,):
        describe = str


class ImportedInClassBody(Base):
    def describe(self):
        return super().describe()

    from os import path as describe


class ReadByOtherMethod(Base):
    def describe(self):
        return super().describe()

    def label(self):
        return describe()


class BoundByOtherMethod(Base):
    def describe(self):
        return super().describe()

    def label(self):
        describe = str
        return describe()
