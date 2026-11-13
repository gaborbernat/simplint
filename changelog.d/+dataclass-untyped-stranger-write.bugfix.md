`dataclass-frozen` and `dataclass-slots` no longer offer a safe fix when code sets an attribute no class of the run
declares on an object of unknown class, such as `node.extra = 1` on a loop variable, which may be an instance of the
dataclass.
