import inspect

from package.sub.module import run


def start_remote_session(gateway):
    return gateway.remote_exec(serve_remote_session)


def serve_remote_session(channel):
    from package.sub.module import handle

    handle(channel.receive())


def remote_source():
    return inspect.getsource(build_remote_payload)


def build_remote_payload():
    if run:
        from package.sub import payload

        return payload
    return None


def local_helper():
    from package.sub.module import handle

    return handle


REMOTE_DEBUG = False
if REMOTE_DEBUG:
    pass
if __name__ in {"package.sub.remote_worker"}:
    pass


def dispatch_remote():
    return [start_remote_session][0](dispatched_payload)


def dispatched_payload():
    from package.sub.module import handle

    return handle
