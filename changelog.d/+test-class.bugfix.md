`test-class` (SMP801) offers a fix for a generic class. Each method keeps the class's type parameters that its
signature, body or nested `type` statements use, with their bounds and defaults, so `class TestBox[T]` becomes
`def test_box_default[T](...)`. Defaults need Python 3.13, so below it a method using a defaulted parameter gets no fix.
