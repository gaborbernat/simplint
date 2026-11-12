__all__ = []

from functools import cached_property


class Model:
    @classmethod
    @property
    def kind(cls):
        return "model"

    @classmethod
    @cached_property
    def cached_kind(cls):
        return "model"

    @property
    @staticmethod
    def size():
        return 1

    @cached_property
    @classmethod
    def total(cls):
        return 1

    @property
    def name(self):
        return "model"

    @staticmethod
    def build():
        return Model()


@classmethod
def from_dict(cls, data):
    return cls(**data)
