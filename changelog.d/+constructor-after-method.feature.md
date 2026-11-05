`constructor-after-method` (SMP203) reports a constructor (`__new__`, `__init__`, `__post_init__` or
`__init_subclass__`) defined below another method of its class, and moves it above that method. `definition-before-use`
(SMP201) no longer moves a constructor below a method that calls it, so the two fixes agree.
