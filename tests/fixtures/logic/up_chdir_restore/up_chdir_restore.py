import os


def build(target, fail):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(fail)
    finally:
        os.chdir(old)


def shared_old(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)
    print(old)


def other_restore(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(target)


def other_saved(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        other(old)


def attribute_target(self, target):
    self.old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(self.old)


def not_a_call(target):
    old = 1
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def not_a_change(target):
    old = os.getcwd()
    print(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def change_without_argument(target):
    old = os.getcwd()
    os.chdir()
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def change_with_keyword(target):
    old = os.getcwd()
    os.chdir(target, extra=1)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def saved_with_argument(target):
    old = os.getcwd(1)
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def other_save(target):
    old = other()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def other_change(target):
    old = os.getcwd()
    other(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def other_reset(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        other(old)


def with_except(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    except ValueError:
        return None
    finally:
        os.chdir(old)


def comment_in_body(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)  # keep
    finally:
        os.chdir(old)


def comment_before_finally(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    # keep
    finally:
        os.chdir(old)


def one_line_body(target):
    old = os.getcwd()
    os.chdir(target)
    try: compile_project(False)
    finally:
        os.chdir(old)


def outer_restore(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project(False)
    finally:
        os.chdir(old)


def expression_change(target):
    old = os.getcwd()
    target
    try:
        return compile_project(False)
    finally:
        os.chdir(old)
