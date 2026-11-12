from unawaited_coroutine_lib import shuffle


def shutdown(cancel, cancel_msg, exc_type=None):
    return cancel, cancel_msg, exc_type


def move(source, target):
    return source, target


def triple(first, second, third):
    return first, second, third


@staticmethod
def decorated(first, second, third):
    return first, second, third


class Pool:
    def stop(self, cancel, cancel_msg, exc_type=None):
        return cancel, cancel_msg, exc_type

    def close(self, cancel, cancel_msg):
        self.stop(cancel, cancel, cancel_msg)
        self.stop(cancel, cancel_msg)


def caller(cancel, cancel_msg, source, target, first, second, third, rest):
    shutdown(cancel, cancel, cancel_msg)
    triple(third, first, second)
    shuffle(second, third, first)
    shuffle(second, second, first)
    move(target, source)
    triple(second, first, third)
    triple(first, second, third)
    triple(*rest)
    triple(third, first)
    decorated(third, first, second)
    unknown(third, first, second)
