`single-user-constant` (SMP202) no longer inlines a constant that a doctest of its own module reads, which would leave
the example reading a name that is gone.
