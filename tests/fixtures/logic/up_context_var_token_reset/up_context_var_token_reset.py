import contextvars

request_id = contextvars.ContextVar("request_id")
user = contextvars.ContextVar("user")


def simple(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.reset(token)


def used_after(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.reset(token)
    print(token)


def used_in_body(value):
    token = request_id.set(value)
    try:
        print(token)
    finally:
        request_id.reset(token)


def commented(value):
    token = request_id.set(value)
    try:
        return process()  # keep
    finally:
        request_id.reset(token)


def comment_before_try(value):
    token = request_id.set(value)
    # keep
    try:
        return process()
    finally:
        request_id.reset(token)


def one_line_body(value):
    token = request_id.set(value)
    try: process()
    finally:
        request_id.reset(token)


def with_handler(value):
    token = request_id.set(value)
    try:
        return process()
    except ValueError:
        return None
    finally:
        request_id.reset(token)


def with_else(value):
    token = request_id.set(value)
    try:
        return process()
    except ValueError:
        return None
    else:
        request_id.reset(token)


def extra_finally(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.reset(token)
        print("done")


def other_variable(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        user.reset(token)


def other_argument(value, saved):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.reset(saved)


def release(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        release(token)


def unset(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.unset(token)


def no_argument(value):
    token = request_id.set()
    try:
        return process()
    finally:
        request_id.reset(token)


def set_other_method(value):
    token = request_id.make(value)
    try:
        return process()
    finally:
        request_id.reset(token)


def plain_value(value):
    token = value
    try:
        return process()
    finally:
        request_id.reset(token)


class Holder:
    def store(self, value):
        self.token = request_id.set(value)
        try:
            return process()
        finally:
            request_id.reset(self.token)


def no_finally(value):
    token = request_id.set(value)
    try:
        return process()
    except ValueError:
        return None


def bare_finally(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        token
