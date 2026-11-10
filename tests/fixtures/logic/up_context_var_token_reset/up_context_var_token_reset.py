import contextvars
from contextvars import ContextVar

request_id = contextvars.ContextVar("request_id")
user = contextvars.ContextVar("user")
tenant: ContextVar[str] = ContextVar("tenant")


class Settings:
    def set(self, value):
        old = self.value
        self.value = value
        return old

    def reset(self, old):
        self.value = old


settings = Settings()


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


def annotated_module_variable(value):
    token = tenant.set(value)
    try:
        return process()
    finally:
        tenant.reset(token)


def annotated_parameter(variable: ContextVar[str], value):
    token = variable.set(value)
    try:
        return process()
    finally:
        variable.reset(token)


def unannotated_parameter(variable, value):
    token = variable.set(value)
    try:
        return process()
    finally:
        variable.reset(token)


def other_class(value):
    token = settings.set(value)
    try:
        return process()
    finally:
        settings.reset(token)


def attribute_receiver(holder, value):
    token = holder.variable.set(value)
    try:
        return process()
    finally:
        holder.variable.reset(token)
