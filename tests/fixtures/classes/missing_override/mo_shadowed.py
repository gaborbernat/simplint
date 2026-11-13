from mo_base import MoBase


def override(method):
    return method


class MoShadowed(MoBase):
    def run(self) -> int:
        return 10


print(MoShadowed)
