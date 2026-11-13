def _called_lambda_helper():
    return {}


options_snapshot = (lambda: _called_lambda_helper())()


def _skip_reason(case):
    return case == "slow"


suite_cases = build_suite(skip=lambda case: _skip_reason(case))


def _rx_level(level):
    return f"level{level}"


class LevelLexer:
    def state_for(level):
        return (_rx_level(level), level)

    tokens = [state_for(1), state_for(2)]


def _render_cell(cell):
    return str(cell)


def render_row(row):
    return [_render_cell(cell) for cell in row]


def _bound_lambda_helper():
    return 1


bound_lambda = lambda: _bound_lambda_helper()  # noqa: E731


def _method_helper():
    return 2


class NeverCalledInBody:
    def run(self):
        return _method_helper()


def _base_helper():
    return 3


class Base:
    def run(self):
        return _base_helper()


class Child(Base):
    pass


LIMIT = 3


def _default_helper():
    return 4


def with_default(count=LIMIT):
    return _default_helper() + count


def main():
    return _limited()


def _limited(count=LIMIT):
    return count


def _from_constant(count=LIMIT):
    return count


def entry():
    return _from_constant()


def _scaled_part():
    return 2


def _scaled():
    return _scaled_part() * 2


SCALED = _scaled()


def _left():
    return 1


def _right():
    return 2


def pick(sides=(_left, _right)):
    return [side() for side in sides]


def _shared_step():
    return 5


def _first_step():
    return _shared_step()


def _second_step():
    return _shared_step()


STEPS = (_first_step(), _second_step())
