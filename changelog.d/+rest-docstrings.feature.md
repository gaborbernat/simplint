The new reStructuredText rules, SMP310 to SMP323, check reStructuredText field lists in docstrings when
`docstring-style = "rest"`: every field is well formed, described with more than its name and separated from the text
above by a blank line, documents a parameter or attribute that exists, with `:ivar:` or `:cvar:` as fits, once, and
carries no type, which belongs in annotations; roles keep their backticks and inline literals close; a public function
or class documents each parameter, attribute, return or yield value and exception it raises itself, and no internal
name. Their fixes add each missing field with an empty description for you to fill in, and remove or correct fields for
what no longer exists.
