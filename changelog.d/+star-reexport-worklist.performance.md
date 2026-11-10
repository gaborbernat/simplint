Following names through chains of `import *` re-exports looks each name up in a set of the source's `__all__` and
revisits only the modules that gained names, so a chain of 200 modules whose `__all__` lists the whole chain takes 0.14
seconds instead of 6.8.
