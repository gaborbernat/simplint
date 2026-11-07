Entry points that name a module without `:attribute`, such as a pytest plugin, were ignored. Every public name of such a
module now counts as public API, so `function-docstring` (SMP901) no longer asks for docstrings on its hooks and
fixtures, and `undocumented-public-api` (SMP903) checks them whatever `__all__` lists.
