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
