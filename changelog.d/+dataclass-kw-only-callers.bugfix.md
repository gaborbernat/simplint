`dataclass-kw-only` (SMP114) no longer breaks positional calls such as `Partition(msg.topic(), msg.partition())`: its
fix passes the arguments of the calls in the same module by keyword, and reports each positional call in other checked
files with a safe fix of its own, which the same run applies. The fix of the decorator is not offered while a use could
still pass fields by position, such as a call unpacking `*args` (`**kwargs` passes by keyword and does not block it), a
subclass, a star import of the module, or a call in a file that ignores the rule.
