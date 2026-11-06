from _unused_return_value import RetBuilder, RetWriter, ret_dropped, ret_passed, ret_read, ret_tests_read

HANDLER = ret_passed


def run(data: str) -> int:
    ret_dropped(1)
    ret_tests_read(1)
    RetBuilder().ret_build()
    RetWriter().ret_commit(data)
    return ret_read(1)
