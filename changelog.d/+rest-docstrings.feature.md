The new `SMP31` rules check reStructuredText field lists in docstrings when `docstring-style = "rest"`: every field is
well formed and described, documents a parameter or attribute that exists, and carries no type, which belongs in
annotations; a public function or class documents each parameter, attribute, return or yield value and exception it
raises itself. Their fixes add each missing field with an empty description for you to fill in, and remove fields for
what no longer exists.
