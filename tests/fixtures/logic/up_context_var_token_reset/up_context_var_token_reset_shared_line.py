import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    count = 0; token = request_id.set(value)
    try:
        run()
    finally:
        request_id.reset(token)
    return count
