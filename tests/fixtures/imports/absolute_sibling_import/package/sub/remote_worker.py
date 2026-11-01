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
