A `from` import in a `.pth` file, such as `import sys; from pkg.hook import value`, counts the name it imports as read
at startup, as `import pkg.hook; pkg.hook.value` does, so `value` is no longer reported as test-only or unused.
