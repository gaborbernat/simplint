# Changelog

## 0.10.0 (2026-11-10)

<a name="release-0-10-0"></a>

<a name="release-0-10-0-added"></a>

### Added

- `unused-ignore` reports unused `simplint: ignore` selectors and removes them with a safe fix
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `entrypoint-module-name` reports command entrypoints in `main.py` or `_main.py` so they can move to `__main__.py`
  ([#76](https://github.com/gaborbernat/simplint/pull/76)) by [@gaborbernat](https://github.com/gaborbernat).
- `unguarded-entrypoint-call` uses script metadata, shebangs, executable bits, and `__main__.py` to report trailing
  entrypoint calls regardless of their name. Single-line statements have an unsafe guard fix
  ([#78](https://github.com/gaborbernat/simplint/pull/78)) by [@gaborbernat](https://github.com/gaborbernat).
- `private-import-path` reports private module-object imports and private imports outside a direct relative path,
  including nested names such as `._x._y`. Tests may import names from a private module only when their package path and
  `test_` stem match the source module ([#74](https://github.com/gaborbernat/simplint/pull/74)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-0-changes"></a>

### Changes

- Removed `up-unittest-assert-helper`: Ruff's
  [`pytest-unittest-assertion`](https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/) reports every assertion it
  covered when Ruff rules are all enabled ([#75](https://github.com/gaborbernat/simplint/pull/75)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-0-fixed"></a>

### Fixed

- `constant-assert` leaves tuple and string literal assertions to Ruff's F631 and PLW0129 even when simplint could fix
  them ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` leaves `mixedCase` names to Ruff's N816 even when simplint could rename them
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` leaves `yield from` assignments to Ruff's RUF070 even when Ruff's fix differs
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` respects Ruff's enabled local-variable limit before suggesting moving a constant into a
  function ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` reports private module attribute access even when Ruff selects `PLC2701`, which checks
  imports rather than attribute access ([#74](https://github.com/gaborbernat/simplint/pull/74)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` leaves `__main__.py` out of the missing-export check
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `init-module-content` accepts package re-exports listed in `__all__`, including in applications and private
  subpackages ([#73](https://github.com/gaborbernat/simplint/pull/73)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `init-module-content` allows test package initializers, PEP 562 hooks, and `TYPE_CHECKING` blocks
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-caller-helper` avoids suggesting inlining when it would cross enabled Ruff limits for nested blocks or try
  statements, or trigger [raise-within-try](https://docs.astral.sh/ruff/rules/raise-within-try/)
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `long-tuple-return` respects `@override` return contracts when a method returns a long tuple
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `duplicate-code` compares duplicate code only within the same distribution
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `callable-ellipsis` allows callable annotations that a test body does not invoke
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- `stdlib-mock` allows `create_autospec` imports without other patching helpers
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-0-deprecated"></a>

### Deprecated

- Remove
  [long-comment-line](https://github.com/gaborbernat/simplint/blob/1a1b43c/src/rules/comments/long_comment_line.md):
  Ruff's [E501](https://docs.astral.sh/ruff/rules/line-too-long/) and
  [W505](https://docs.astral.sh/ruff/rules/doc-line-too-long/) report every case it covered
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- Remove [star-import](https://github.com/gaborbernat/simplint/blob/1a1b43c/src/rules/imports/star_import.md): Ruff's
  [F403](https://docs.astral.sh/ruff/rules/undefined-local-with-import-star/) reports every case it covered
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.9.0 (2026-11-09)

<a name="release-0-9-0"></a>

<a name="release-0-9-0-added"></a>

### Added

- `simplint: ignore[...]` accepts rule names such as `imprecise-annotation`. `numeric-ignore` (SMP008) reports known
  numeric codes in these comments and offers a safe fix to replace them with names. `--add-ignore` writes names too
  ([#71](https://github.com/gaborbernat/simplint/pull/71)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-9-0-fixed"></a>

### Fixed

- `unused-export` and related API rules count functions and classes named by `pkgutil.resolve_name`, argparse handler
  defaults, and supported `pyproject.toml` tool settings as used. This includes setuptools attributes and command
  classes, mypy, Pylint, pytest and Poetry plugins, and local PDM build hooks
  ([#71](https://github.com/gaborbernat/simplint/pull/71)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.8.0 (2026-11-08)

<a name="release-0-8-0"></a>

<a name="release-0-8-0-added"></a>

### Added

- `dataclass-kw-only` (SMP703) reports a dataclass once, at its definition, and lists its positional calls as related
  locations instead of reporting each call. The fix is offered only when every such call is in the same file; calls in
  other files need converting by hand ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unexplained-regex` (SMP533) reports a regular expression pattern of four or more parts, counting its groups,
  alternations, character classes with their repeats and repeats of groups, that has no comment: neither a `#` comment
  in verbose mode nor a `(?#...)` comment ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unnamed-regex-group` (SMP532) reports a regular expression pattern with two or more unnamed capture groups, or with
  one that the code reads by its number, so each group gets a name the code can read. The groups a pattern's code never
  reads become non-capturing under an unsafe fix ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-8-0-changes"></a>

### Changes

- `all-format` (SMP405) no longer reports a list whose closing bracket follows a line break with no trailing comma when
  the project's Ruff configuration selects [COM812](https://docs.astral.sh/ruff/rules/missing-trailing-comma/), which
  reports it. A list whose last name is parenthesized and followed by a comma is no longer reported as missing that
  comma ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `blanket-ignore` (SMP004) leaves the `ruff: ignore` and `ruff: file-ignore` comments without codes to Ruff's
  [RUF103](https://docs.astral.sh/ruff/rules/invalid-suppression-comment/), which reports them, when the project's Ruff
  configuration selects it. The other forms are still reported here
  ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `class-member-order` (SMP203) no longer reports the statement of a Django model that
  [DJ012](https://docs.astral.sh/ruff/rules/django-unordered-body-content-in-model/) reports, when the project's Ruff
  configuration selects it, unless that report carries the fix for the class, which DJ012 lacks. Properties, class
  methods and other members it reports are still reported here ([#67](https://github.com/gaborbernat/simplint/pull/67))
  by [@gaborbernat](https://github.com/gaborbernat).
- `collection-built-by-statements` (SMP531) no longer reports a list that
  [FURB113](https://docs.astral.sh/ruff/rules/repeated-append/) reports, when Ruff's preview mode is on and the
  project's Ruff configuration selects it ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `constant-assert` (SMP804) no longer reports a string, bytes or f-string test it cannot delete when the project's Ruff
  configuration selects [PLW0129](https://docs.astral.sh/ruff/rules/assert-on-string-literal/), which reports it
  ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `error-message-matching` (SMP507) no longer reports a comparison inside an `assert` that names the exception when the
  project's Ruff configuration selects [PT017](https://docs.astral.sh/ruff/rules/pytest-assert-in-except/), which
  reports it ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) no longer reports an `Any` that [ANN401](https://docs.astral.sh/ruff/rules/any-type/)
  reports when the project's Ruff configuration selects it, now also inside `Optional`, `Union`, `|` and `Annotated`.
  While ANN401 runs, it reports the `Any` ANN401 skips, such as `self: Any`, `@override` signatures and, under
  `allow-star-arg-any`, `*args: Any` ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `init-module-content` (SMP407) no longer reports the statements of an `__init__.py` that
  [RUF067](https://docs.astral.sh/ruff/rules/non-empty-init-module/) reports, when Ruff's preview mode is on and the
  project's Ruff configuration selects it ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `sleep-in-test` (SMP808) no longer reports an `await asyncio.sleep` that is the whole body of a `while` loop when the
  project's Ruff configuration selects [ASYNC110](https://docs.astral.sh/ruff/rules/async-busy-wait/), which reports it
  ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP503) no longer reports the message of a logging call when the project's Ruff configuration
  selects [G003](https://docs.astral.sh/ruff/rules/logging-string-concat/), which reports it, nor two literals joined
  across lines when it selects [ISC003](https://docs.astral.sh/ruff/rules/explicit-string-concatenation/), which reports
  it ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409) no longer reports a private protocol or TypedDict class that the file never reads when
  the project's Ruff configuration selects [PYI046](https://docs.astral.sh/ruff/rules/unused-private-protocol/) or
  [PYI049](https://docs.astral.sh/ruff/rules/unused-private-typed-dict/), which reports it
  ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-re-prefixmatch` (SMP522) no longer reports a `re.match` call with a plain-text pattern whose result is only tested
  for truth when the project's Ruff configuration enables preview mode and selects
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/), which rewrites it to `str.startswith`
  ([#67](https://github.com/gaborbernat/simplint/pull/67)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-unittest-assert-helper` (SMP811) stops with a conflict error when the project's Ruff configuration selects
  [PT009](https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/), since both report the same `self.assertTrue`
  and `self.assertFalse` calls and fix them in opposite ways ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-8-0-fixed"></a>

### Fixed

- `absolute-sibling-import` (SMP302) no longer reports or fixes a project that runs flake8-lazy, configured by a
  `[tool.flake8-lazy]` table or a pre-commit or prek hook with id `flake8-lazy`. Its fix made flake8-lazy rewrite each
  module's `__lazy_modules__` list into entries that `ty` rejects
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` keeps an annotated constant as an annotated local instead of inlining its literal, so a value
  such as a dict under `Final[dict[str, list[str]]]` keeps the type it declares and still type checks
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP901) reads a method documented by an `.. autoclass::` directive with `:members:` under its
  class name, so its docstring is no longer reported as internal
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP901) and `unused-definition` (SMP409) read `.. autofunction::` directives that name a
  function of a package by the package, as in `app.helper` for `app/__init__.py`, as documentation
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409) and `test-only-code` (SMP408) keep a function that the documentation names with an
  autodoc directive, so a test calling it no longer makes it test-only code
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `automodule` of a package documents the names its `__init__.py` defines, so they are no longer reported as unused
  definitions when a `.rst` or `.md` page documents them with `:members:`
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- Entry points that name a module without `:attribute`, such as a pytest plugin, were ignored. Every public name of such
  a module now counts as public API, so `function-docstring` (SMP901) no longer asks for docstrings on its hooks and
  fixtures, and `undocumented-public-api` (SMP903) checks them whatever `__all__` lists
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` (SMP401) no longer lists `annotations` from `from __future__ import annotations` as a name the
  other modules import from a bare module entry point, such as a pytest plugin. A `__future__` import binds no name its
  users read ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `blanket-ignore` (SMP004) no longer reports `# ruff: ignore [E501]` and `# ruff: file-ignore [E501]`, which name codes
  with a space before the bracket, as Ruff reads them ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-export`, `unexported-public-name` and `unused-definition` no longer treat the hooks of the `[build-system]`
  build backend, such as `build_wheel`, as unused or private. A build frontend calls those hooks by name, so they are
  public names, as entry points are ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) no longer reports `object` as the return type of a `Callable`, such as
  `Callable[..., object]`, which says that the result is ignored. `object` in its parameters is still reported
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-prefix`, `module-variable-case`, `unexported-public-name`, `single-user-constant` and
  `module-attribute-import` keep their reports but offer no fix for a module that a write under a computed name may set,
  such as `setattr(module, name, value)` or `mock.patch(f"pkg.module.{name}")`. Renaming, moving or inlining its
  variables would break that write ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `conditional-in-test` (SMP803) no longer reports a conditional expression that is the item of a comprehension, such as
  `[tag if tag else "-" for tag in tags]`, as it already did for `if` clauses in comprehensions. A conditional nested
  inside such an item is still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` (SMP402), `unexported-public-name` (SMP403), `unused-definition` (SMP409) and `test-only-code`
  (SMP408) no longer report the `coverage_init` function of a module that `[tool.coverage.run] plugins` in
  `pyproject.toml` lists, since coverage calls it by name. The module is found in the project's directory or in a
  `pythonpath` (pytest) or `extra-paths` (ty) directory ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` (SMP701), `dataclass-slots` (SMP702) and `dataclass-kw-only` (SMP703) no longer offer a fix for a
  dataclass decorator that holds a comment, since rewriting the decorator dropped the comment
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` (SMP701) no longer offers `frozen=True` for a class whose instances get a `__dict__` store, as
  `x.__dict__ = ...`, `del x.__dict__` or `setattr(x, "__dict__", ...)`, since a frozen instance raises
  `FrozenInstanceError` on it and the fix broke the code it changed. Reading `__dict__`, as `vars(x)` does, keeps the
  fix ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` (SMP701) no longer reports a dataclass written to after construction when the plain class it would
  become is one Ruff rejects: B903 (`class-as-data-structure`) when the class has no base classes or class keywords, no
  decorator besides the dataclass's, no method but private ones, and no factory default or `init=False` field, or
  PLR0913 (`too-many-arguments`) when its parameters, excluding `init=False` fields and `KW_ONLY`, exceed `max-args`.
  Those rules were the only other way to satisfy the rule, so the two rules asked for contradictory code. Other
  dataclasses are still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen`, `dataclass-kw-only` and `dataclass-slots` leave a dataclass alone when a package without
  `__all__`, or with a dynamic one, re-exports it and another file imports it from the package, as the other public API
  rules do ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-kw-only` (SMP703) no longer reports a dataclass that is part of a library's public API, nor the positional
  calls of it in other files, since those callers would break when the fields became keyword-only
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen`, `dataclass-kw-only` and `dataclass-slots` leave a public dataclass alone when a public signature
  of the library returns it, as they do a dataclass its `__all__` lists
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` (SMP701) and `dataclass-slots` (SMP702) treat a `**` splat that sets `frozen` or `slots` to
  something other than `True` as an explicit option, so the fix for the other option is marked unsafe, as it already was
  for a keyword ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` (SMP701), `dataclass-slots` (SMP702) and `dataclass-kw-only` (SMP703) read the options a `**` splat
  passes from a module-level dict, so `@dataclass(**DC_KW)` no longer gets a fix that repeats `frozen=` or `slots=` and
  breaks imports. A `**` splat simplint cannot read leaves the class unreported, and a call whose splat repeats a field
  is no longer rewritten to pass that field by keyword ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` (SMP201) names the definition a misplaced helper should directly follow. A helper placed after
  an earlier helper of the same caller was reported as following the caller
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP903), `function-docstring` (SMP901) and `test-only-code` (SMP408) no longer count a name
  that another module imports from a library's public module when the module's `__all__` leaves the name out, or when
  the module supplies names through a module-level `__getattr__`
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` (SMP202), `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer keep
  a name that only a docstring mentions, since a string spells a name for a patch or a lookup only outside docstrings
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) reports the `module.Name` reads of a module that a doctest reads a name of again,
  since a doctest reads a name rather than patching the module ([#69](https://github.com/gaborbernat/simplint/pull/69))
  by [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` (SMP402) keeps a name that a doctest imports, and its fix no longer removes the name and the import
  the doctest uses ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `__import__("pkg.mod")` returns the package `pkg`, so the names read off its result are read from `pkg` and not from
  `pkg.mod`, unless its `fromlist` is a non-empty list or tuple, or an expression that may hold names. A submodule read
  off its result, as in `__import__("pkg.mod").mod.name`, is read from `pkg.mod`
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` reports `module.Name` access again when another file reads an attribute of a module with the
  same name through `__import__` or a plain `import`. Such a read used to suppress the report for every module of that
  name ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` and `module-variable-prefix` no longer rename a name that a module reads off the result of
  `importlib.import_module`, `__import__` or `pytest.importorskip` called with a string literal, directly or through a
  variable ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `rst-missing-param`, `rst-missing-attribute`, `rst-missing-raises` and `rst-returns` no longer offer to add `:param:`,
  `:ivar:`, `:raises:`, `:returns:` or `:yields:` fields with empty descriptions, which Sphinx's nitpicky builds reject
  and `rst-empty-field` reports. The reports stay, and the author writes the descriptions
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer crashes a debug build when its fix inlines an empty string into an f-string
  field. The fix deletes the field instead ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` (SMP403) and `unused-definition` (SMP409) no longer report the names of an entry point's
  module when that module sits in the project's root directory and `src` does not list that directory
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `error-message-matching` (SMP507) no longer reports a comparison of the text of two exceptions, such as
  `str(exc) == str(other)` with `other` annotated with an exception type
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP901) no longer copies reST fields and `Args:` or `Parameters` sections into the comment its
  fix writes, since the signature documents them ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409) no longer reports the hook class of a hatchling custom build or metadata hook, such as
  `_CustomBuildHook` in `hatch_build.py`, as unused. hatchling calls that class, and the `path` of the hook is honored
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `--fix --unsafe-fixes` no longer stops with "Failed to converge after 100 iterations" and applies nothing when a file
  has about 100 identical fixes, such as `single-use-variable` inlines of `lock = build()`, and one of them is
  withdrawn. The withdrawn fix stays withheld while the others apply
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) no longer reports `object` as the annotation of `*args` or `**kwargs`, the honest type
  for arbitrary values. Before, no annotation satisfied both it and ANN401, which flags `Any` there
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP901) leaves the docstring of a method that a public class of a library inherits, without
  overriding it, from a private base or mixin, since users reach it through that class. `undocumented-public-api`
  (SMP903) expects one there ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP903) and `function-docstring` (SMP901) judge the methods a public class inherits from a
  private base as public API when the library re-exports the class from a private module
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP903) and `function-docstring` (SMP901) judge the methods a private class inherits from a
  base of its module as public API when a public signature of the library returns an instance of that class
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `--isolated` no longer reads the `pyproject.toml` of a nested test project, so an invalid one no longer fails the run
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` (SMP401) keeps reporting a name that other modules import but `__all__` leaves out in a public
  module of a library, and offers no fix there: listing it widens the library's API and what `import *` binds
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-slots` (SMP702) and `dataclass-frozen` (SMP701) no longer report the public dataclasses of a library,
  including those nested in its public classes, since `slots` and `frozen` change what users can do with an instance
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-default` (SMP506) no longer removes a default from a public function or class of a library's public module,
  since code outside the project may call it without the argument. Its hooks and entry points were already left alone
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- The fixes of `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer rename a name that a
  public module of a library offers or lists in `__all__`, since its users import it
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-final` (SMP601) no longer reports the module variables of a Sphinx `conf.py`, which Sphinx reads by their
  names, as `module-variable` already leaves them alone ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) no longer reports a read of a private name, such as `main._compute_max_logs`, when
  Ruff's `import-private-name` (PLC2701) would flag the import it suggests, so the two rules no longer need a
  suppression to agree ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) no longer reports an attribute read off a parameter, local, loop or comprehension
  variable named like the module it imports. Only the reads that resolve to the module are reported, and its fix
  rewrites only those reads. A module whose name a local, parameter, closure variable or class attribute rebinds used to
  be skipped entirely; its real reads are now reported and fixed
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) and `missing-final` (SMP601) count a `setattr` or `patch.object` in a loop over a
  literal tuple or list of modules as patching the name on each module the loop names. Such a module keeps its attribute
  on the module instead of being reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report a name that one module-level
  `if`, `try` or `with` block binds and reads, such as `msg` in a version check before a `raise`. A name read outside
  the block, or by another file, is still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report a module-level alias of a
  function the module defines, such as `_build_cli = build_cli`, which follows the function's name. An alias of an
  imported function is still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report an annotation without a value
  under `if TYPE_CHECKING:`. Such a declaration types a name that a module's PEP 562 `__getattr__` serves at runtime.
  Renaming it broke the type checker's view of `module.log`, while `__getattr__` keeps serving it at runtime. An
  assigned name in the same block is still checked ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` and `module-variable-prefix` keep their report but offer no rename for a module variable that
  another module assigns as an attribute, as `module.NAME = value` or `setattr(module, "NAME", value)` does, since the
  rename would leave that write pointing at the old name ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-final` (SMP601) no longer reports a module variable that a `setattr` call sets on its module, such as
  `scoped.setattr(paths, "OVERRIDE_PIPX_HOME", None)` in a test or `monkeypatch.setattr("pkg.paths.NAME", None)`,
  whatever the object is named. A module that a `setattr` call may set under a computed name, such as
  `setattr(paths, name, value)`, reports none of its variables, since any of them may be reassigned
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- The call-site rules, such as `unused-return-value` (SMP411), count a call made through a module that neither the run
  nor the standard library of the newest Python provides, such as `log.set_threshold(...)` after
  `from distutils import log` in a tree that shims `distutils.log`, as a call of the run's file of that last name, so a
  function whose value that call uses is not reported. A standard-library module such as `json` never counts
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- A name that another part of the project stores to as an attribute keeps its `missing-final` (SMP601) suppression
  inside a nested test project, as it does outside one ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Results of the rules a nested project leaves out are no longer reused by the outer project, so `simplint check`
  reports the same docstring findings whichever project it runs from
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- The documentation pages of a nested test project no longer count as the run's project's documentation, so a name only
  those pages give is reported as unused again ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A library directory whose name only starts with `test`, such as `testkit`, is no longer checked as a nested test
  project, so the rules that judge its docstrings and names apply to it again
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- Nested test projects, such as the `roots/test-*` directories of Sphinx's test suite, are now checked as projects of
  their own: a name one of them defines is not used by another, and their test-module status, project kind and
  documentation stay within them. A name the run's project reads is used in them, and an `.. automodule::` with
  `:members:` documents the members Sphinx renders, so their names count as used and their docstrings stay. Their files
  are test input, so the docstring rules and the export and naming visibility rules leave them alone, while the other
  rules still check them ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A nested project rooted at a `test` or `tests` directory, such as numpy's `tools/swig/test` with its own `setup.py`,
  has its files treated as test code by every rule that reads test modules, as the directory name says, so
  `duplicate-code` (SMP530) no longer compares them with the rest of the run
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` (SMP202) no longer inlines a constant that a doctest of its own module reads, which would leave
  the example reading a name that is gone ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` (SMP802) no longer reports a row built by a helper of the module whose every path ends in
  `pytest.param(..., id=...)` ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` (SMP802) checks a row name bound once to a `pytest.param(...)` call by that call's `id=`, so
  a module constant that already has an id is no longer reported. Names the rule cannot read, such as imported or
  rebound ones, are skipped ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` (SMP802) counts numbers by the id pytest gives them, so `0x10` and `16`, or `1e3` and
  `1000.0`, repeat each other and their rows are reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` (SMP802) no longer reports a row that is a plain string, number, `True`, `False` or `None`
  whose value pytest already uses as its id, such as `"select"`. It still reports such a row when another row repeats
  its value, since pytest then suffixes the shared id ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` (SMP802) no longer reports a starred generator in a parametrize list, such as
  `*(pytest.param(...) for ...)`, since the rows it unpacks are not known. A starred list or tuple, written inline or
  bound to a name, has its rows checked instead ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` no longer drops a name that a test patches through its dotted path, `mock.patch("pkg.mod.NAME")`, or
  through the module object, `monkeypatch.setattr(module, "NAME", value)`, and `single-user-constant` no longer inlines
  a constant that a string in the run spells, which would replace the value the test patches
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `--fix` withdraws only the fix whose result Ruff would report, where it used to withdraw every fix of that rule in the
  file, so the other fixes of the rule still apply ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-default` (SMP506) again judges the private methods of a public class in a library's public module. Only the
  public methods of such a class are public API, so the defaults of its private methods are checked
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` (SMP402) no longer reports names in the `__all__` of a public module of a library, since users import
  those names and no module of the project needs to. Private modules and applications keep the report
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` (SMP403) and `module-variable-prefix` (SMP404) still report a name missing from `__all__` in
  a public module of a library, but no longer offer to rename it private, since users import those names. The report
  asks for the name in `__all__` or a rename by hand ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unnamed-regex-group` (SMP532) reads the decorators, default values and annotations of a function in the scope around
  it, so a match read in a default no longer makes the match look unused and gets an unsafe fix that breaks the default
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unnamed-regex-group` (SMP532) and `unexplained-regex` (SMP533) no longer read a pattern as non-verbose when its
  `flags` argument is a name that holds `re.VERBOSE`, so `#` comments are checked as comments and the fix no longer
  writes `(?:` inside one ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` (SMP402) treats every public module of a library as part of its API, which leaves the `api-modules`
  option with nothing to do, so it is gone ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A name a function binds to modules, as `mod = pytest.importorskip(...)` does, reads the names of those modules only,
  so `module-variable-prefix` (SMP404) no longer offers to rename a name a test reads off a module it loads. A local
  variable that shares its name with a module-level one no longer makes that name count as read in other files, which
  reported a variable of an `if` block that only the block reads as `module-variable-case` (SMP102)
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `rst-attribute-kind` (SMP922) and `rst-missing-attribute` (SMP920) take a class-body `Final` with a value as a class
  variable, as PEP 591 defines it, so such a constant takes `:cvar:` rather than `:ivar:`
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- The reStructuredText rules that report a missing parameter, attribute or raised exception (SMP916, SMP920 and SMP924)
  list no fix in the rule reference, as they offer none ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `rst-missing-attribute` (SMP920) no longer reports the attributes that a class defined inside a method sets on its own
  `self` as attributes of the enclosing class ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` and `unused-export` no longer rename or drop a name that a script reads by its string key off
  the globals `runpy.run_path` or `runpy.run_module` returns, such as `runpy.run_path(path)["main"]`, when the path or
  module names a project module. A key that no static path names counts as a read of any module's name
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- SARIF `relatedLocations` give their file as a `file://` URI, as the primary location of the same result does
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409) no longer reports a class or function that a doctest of its module uses without importing
  it, since a doctest runs in the globals of its module ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-caller-helper` (SMP505) no longer asks to inline a helper holding a conditional into a test, where
  `conditional-in-test` (SMP803) would then report it ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-caller-helper` (SMP505) no longer suggests inlining a helper when the caller would then pass the
  cognitive-complexity limit, which would ask to split it again ([#69](https://github.com/gaborbernat/simplint/pull/69))
  by [@gaborbernat](https://github.com/gaborbernat).
- `single-caller-helper` (SMP505) no longer asks to inline a helper when the merged caller could cross the limit of
  Ruff's `C901`, `PLR0912`, `PLR0915` or `PLR0911` while Ruff runs that rule; the inlined counts are estimated
  conservatively ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-caller-helper` (SMP505) no longer suggests inlining a helper annotated `TypeGuard` or `TypeIs`, since the bare
  expression would drop the narrowing its caller relies on ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer inlines a value into a parameter's default when Ruff's
  `mutable-argument-default` (B006) or `function-call-in-argument-default` (B008) runs and reports that value there
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer asks to inline a conditional expression, or a value with a string literal in
  it, into an f-string replacement field, where the result reads worse than the two statements it replaces. A string
  value on its own is still inlined ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` no longer nests an f-string or string literal inside the f-string it is inlined into, which it
  wrote as `f"[{f'<{value}>'}]"`; the value's text joins the enclosing f-string instead, as `f"[<{value}>]"`
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer inlines a value into the `next`, `anext`, `send` or `close` call that reads
  it. Inlined, the call became the only holder of a generator, so its `finally` block ran as soon as the call returned
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` no longer adds parentheses around a value inlined where its precedence does not need them, such
  as the test of a conditional expression, and keeps them where an operator needs them
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` and `unexported-public-name` no longer remove or rename a name a module reads off a module loaded from
  a file path with `importlib.util.spec_from_file_location`, when the path names a project module. Names read off a
  module loaded by `importlib.import_module` and similar calls now count as imports, so `missing-from-all` and
  `unused-export` see them too ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A `from` import in a `.pth` file, such as `import sys; from pkg.hook import value`, counts the name it imports as read
  at startup, as `import pkg.hook; pkg.hook.value` does, so `value` is no longer reported as test-only or unused
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- Names that a `.pth` file reads at interpreter startup, such as `import pkg.hook; pkg.hook._run()`, now count as used
  outside the tests, as entry points do. `test-only-code` (SMP408), `unused-export` (SMP402), `unexported-public-name`
  (SMP403), `unused-definition` (SMP409) and `function-docstring` (SMP901) no longer report them as test-only, unused,
  unlisted or undocumented ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--stdin-filename` judges a file by its path as `check` does, so a helper in a `tests` directory is no longer reported
  as unused when checked from inside that directory ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` no longer moves a module variable that another file assigns or deletes as an attribute of its
  module, as `module.NAME = value` does, which the move would silently break. `module-attribute-import` leaves only the
  attribute another file stores to or a dotted string patches alone, as it does for `setattr`, so the module's other
  attributes are still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP503) no longer reports a chain whose non-literal operand holds a string with a backslash
  when the target is older than Python 3.12, where no f-string replacement field can contain one
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP503) no longer reports a `+` chain with an instance of a class the module defines, such as
  `Markup("<b>") + "<i>"`, since `+` runs the class's own `__add__` and an f-string would change what the code does
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP503) no longer reports a `+` chain with an instance of a class imported into the module,
  such as `Markup("<b>") + "<i>"` with `markupsafe.Markup`, since `+` runs that class's own `__add__`. A class of the
  project is recognized whatever its name, and an import from outside the project counts as a class when its name is
  CapWords ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `suppression-without-reason` (SMP003) accepts a reason written as a comment on the line directly above a trailing
  suppression, such as `# ty: ignore[...]`, so the reason need not share the line with the suppression
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) no longer reports a read through an alias of the project's own top-level package,
  such as `pd.NaT` for `import pandas as pd`. Reads through its submodules, and through an alias equal to the package
  name, are still reported ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` (SMP401) no longer asks to list a name bound only under `if TYPE_CHECKING:`, since listing it makes
  `from module import *` fail at run time ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409), `unexported-public-name` (SMP403), `module-variable-case` (SMP102) and
  `module-variable-prefix` (SMP404) no longer skip a name only because a stdlib or third-party import or a doctest
  spells it, since such a spelling says nothing about the names of the project. Where such a spelling may read the name,
  the rename fix is withheld ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-default` (SMP506) follows the `__all__` of a library's public module, as the other API rules do: a public
  function that `__all__` leaves out now has its default reported when every call in the run passes it
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- `vars-call` (SMP604) no longer reports a mapping built from `vars()` in the same expression, such as
  `dict(vars(obj))`, a dict comprehension over `vars(obj).items()` or `{**vars(obj), ...}`, since the attributes of an
  open-ended object have no key type to name ([#69](https://github.com/gaborbernat/simplint/pull/69)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` (SMP502) no longer offers a fix for an assignment whose `if` is the only statement of an outer `if`
  with a comment in its header, as Ruff would not merge the two past that comment. `--fix` also applies the other
  assignments of the file, which it had withdrawn along with that one
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).
- A workspace member is now judged as a library or an application by its own `pyproject.toml` when that has a
  `[project]` or `[build-system]` table, so a workspace root that only configures simplint no longer makes its library
  members applications. The root's `[tool.simplint]` still sets the rules
  ([#69](https://github.com/gaborbernat/simplint/pull/69)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-8-0-documentation"></a>

### Documentation

- Each rule page's "Other tools" section now says in a sentence or two how the rule relates to Ruff: which Ruff rule
  covers which part, or which Ruff issue or pull request asks for the check. Several pages cited issues and pull
  requests that ask for something else; they are corrected ([#67](https://github.com/gaborbernat/simplint/pull/67)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.7.1 (2026-11-06)

<a name="release-0-7-1"></a>

<a name="release-0-7-1-changes"></a>

### Changes

- `blanket-ignore` (SMP004) leaves the bare `noqa`, `ruff: noqa` and `flake8: noqa` comments to Ruff's PGH004, and bare
  `type: ignore` and `pyright: ignore` to PGH003, when the project's Ruff configuration selects those rules. The `ty`,
  `pyrefly`, `simplint` and `ruff: ignore` forms are still reported here
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `constant-assert` (SMP804) leaves a tuple test it cannot fix, such as `assert (1, 2, 3)`, to Ruff's F631 when the
  project's Ruff configuration selects it. A two-element tuple without a message keeps its report and fix
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) leaves an annotation that is or starts with `Any`, as in `Any | None`, to Ruff's
  ANN401 only when the project's Ruff configuration selects it; without ANN401 it now reports those annotations too
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `mock-assertion-typo` (SMP814) leaves the prefix-less assertion names Ruff's PGH005 reports, such as
  `assert mock.not_called()`, to Ruff when the project's Ruff configuration selects it; with Ruff's preview on, names on
  a line of their own too. Names under `not` or `and`, in call arguments, and every misspelling are still reported here
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) leaves a `mixedCase` module variable it cannot rename to Ruff's N816 when the
  project's Ruff configuration selects it and does not list the name in `pep8-naming` `ignore-names`. Renameable names
  and every `lower_case` name are still reported here ([#65](https://github.com/gaborbernat/simplint/pull/65)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-letter-name` (SMP101) leaves `l`, `I` and `O` on variables, parameters and except names to Ruff's E741, on
  classes to E742 and on functions to E743, when the project's Ruff configuration selects them. Other letters, and these
  letters on imports, `match` captures and type parameters, are still reported here
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) leaves a name returned right after its assignment to Ruff's RET504 only when the
  project's Ruff configuration selects it, and otherwise now reports it; with Ruff's preview on, a `yield from` of the
  name is left to RUF070 when it is selected and writes the same code
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `star-import` (SMP309) leaves a star import its fix cannot rewrite to Ruff's F403 when the project's Ruff
  configuration selects it, and reports only the ones it can expand
  ([#65](https://github.com/gaborbernat/simplint/pull/65)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP903) leaves the classes, methods and functions of a public module to Ruff's D101, D102,
  D103 and D106 when the project's Ruff configuration selects them. Public variables, and definitions a private module
  makes and a public module re-exports, are still reported here ([#65](https://github.com/gaborbernat/simplint/pull/65))
  by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-1-fixed"></a>

### Fixed

- The run after the one that creates the cache directory reuses the documentation it read instead of sometimes walking
  the documentation again ([#60](https://github.com/gaborbernat/simplint/pull/60)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-1-documentation"></a>

### Documentation

- Each rule page with a related Ruff rule, issue or pull request names it under Other tools, with its state and how the
  rule differs ([#63](https://github.com/gaborbernat/simplint/pull/63)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The rules table keeps its rows compact on phones, opens a rule from a click anywhere on its row, marks cross-file
  rules in a column of their own, and shows every filter option in full
  ([#57](https://github.com/gaborbernat/simplint/pull/57)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-1-performance"></a>

### Performance

- Runs are faster: a run over unchanged files skips building the import index and answers from the cache, reports are
  written in parallel in every output format, the cache stores fixes and the `full` text of reports ready to use, and
  the process exits without freeing what it built. On pandas, scipy, SQLAlchemy and CPython, a second run takes half the
  time or less, a run without a cache 11 to 25% less, and a second run with `--output-format json` a quarter of the
  time. The cache grows by about the size of the `full` report it keeps
  ([#62](https://github.com/gaborbernat/simplint/pull/62)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.7.0 (2026-11-06)

<a name="release-0-7-0"></a>

<a name="release-0-7-0-added"></a>

### Added

- `catch-all-module` (SMP104) reports modules and packages named `utils`, `helpers` and other grab-bag names, from the
  `naming.catch-all-modules` list ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `cognitive-complexity` (SMP529) reports a function whose cognitive complexity, as SonarSource defines it, is over the
  limit. The score counts each branch, loop and handler more the deeper it is nested, so the rule reads the nesting that
  Ruff's cyclomatic complexity check ignores. The limit is 20 by default and is set with `max-cognitive-complexity`
  under `[tool.simplint.logic]` ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `collection-built-by-statements` (SMP531) reports an empty dict, list or set that the statements right after it only
  fill with entries, and writes those entries in the literal. The fix is safe when every key and value is a constant,
  and unsafe when one is a name, since a name that is not defined would leave the collection partly filled
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `duplicate-code` (SMP530) reports six or more statements in a row that another module repeats, comparing their tokens,
  so comments, layout, imports and docstrings do not count. Test modules and stubs are left out, and
  `logic.min-duplicate-statements` sets how many statements a copy needs. It offers no fix
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `early-assignment` (SMP526) reports a local assigned a constant, such as a number, string or a list of them, above the
  statements that do not use it, when its first use comes further down the same block. Moving the assignment down to
  that use keeps the value beside the code that reads it. It stays put in a function that reads names by their text,
  such as with `exec`, `eval`, `locals` or `six.exec_`, and when a string between the assignment and the use spells the
  name ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `import-cycle` (SMP308) reports a runtime import that closes a cycle of modules importing each other when they load.
  The report names the cycle and sits on the import that closes it. Imports in a function, under `if TYPE_CHECKING:`, a
  `__main__` guard, a false constant or a version check the project's target never passes, and `except ImportError:`
  fallbacks stay out of it, as do test modules ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `long-comment-line` (SMP007) rewraps comment and docstring paragraphs with a line past the comment width, the lines
  Ruff's `E501` and `W505` report but cannot fix ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `long-tuple-return` (SMP528) reports a function that returns a tuple of more elements than the limit, which is two by
  default, since a tuple of three or more values is a record whose fields a dataclass or a named tuple would name. The
  limit is set with `max-tuple-elements` under `[tool.simplint.logic]`
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `loop-in-test` (SMP813) reports a `for` loop in a test function that runs the code under test once per case from a
  table the file lists, such as a literal or a module constant, and asserts on each result, since the first failure
  hides the rest. Parameterize the cases instead. Loops over values the test computes, `range` sweeps, `while` loops,
  loops that only compare items with one result, and assertions inside `self.subTest(...)` or pytest's
  `subtests.test(...)` are left alone ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `meaningless-name` (SMP103) reports variables, parameters and attributes named with a placeholder word such as `foo`,
  from the `naming.meaningless-names` list ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `mock-assertion-typo` (SMP814) reports mock assertion methods written without their `assert_` prefix, such as
  `called_once_with`, or misspelled, such as `assert_called_ones_with`, and renames each to the assertion it stands for.
  The fix is unsafe: a call that checked nothing becomes a check, which may now fail. The unprefixed names return a
  child mock and pass silently on Python 3.11 and older, and so do the unprefixed `awaited` names on every version
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `restating-comment` (SMP005) reports and removes own-line comments that only repeat the code below them
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `star-import` (SMP309) reports `from module import *` when the project defines `module` and the file uses its names,
  and replaces the `*` with the names the file reads from it. The fix is safe when the module lists its exports in
  `__all__`, and unsafe otherwise ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `string-statement-comment` (SMP006) reports triple-quoted strings used as comments and writes them as `#` comments
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `super-forwarding-method` (SMP527) reports a method whose only statement returns the same method of `super()` called
  with its own arguments, such as `return super().load(name)`, when the module defines the parent's method with the same
  parameters. Deleting the method lets the subclass inherit the parent's method. The fix is not offered for an annotated
  method, since its annotations are the types the subclass declares
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `test-only-parameter` (SMP410) reports defaulted parameters that only test calls pass, while production calls leave
  them at their default ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-definition` (SMP409) reports functions, classes, methods and class attributes that nothing in the run uses.
  Public names of a library and names other code may read are left alone, and so are the names the project's Sphinx
  documentation gives through roles, autodoc directives or sphinx-argparse options
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-fixture` (SMP815) reports pytest fixtures that no test or fixture requests, by parameter, by a string such as
  a `usefixtures` or `getfixturevalue` argument, or through the fixture function
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `unused-return-value` (SMP411) reports functions that production code calls without reading the value they return
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-compression-modules` (SMP523) reports imports of `gzip`, `bz2`, `lzma` and `zlib` from Python 3.14, where the
  `compression` package names them. The fix is unsafe: `compression` re-exports the modules, so patches of the old
  module names do not reach the new ones ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-deprecated-stdlib-alias` (SMP525) reports deprecated or removed standard library names that have a replacement,
  such as `threading.currentThread`, `calendar.January` and `configparser.SafeConfigParser`, from the Python version
  that deprecates or removes them. Renames are fixed; names that need a review are only reported
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-re-prefixmatch` (SMP522) reports `re.match()` from Python 3.15, where `re.prefixmatch()` is the same function
  under a clearer name. The fix renames the call ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-utf8-encoding-default` (SMP521) reports `encoding="utf-8"` on `open()`, `io.open()`, `Path` text methods and
  similar calls from Python 3.15, where UTF-8 is the default encoding. Its fix is unsafe: UTF-8 mode can be turned off
  with `PYTHONUTF8=0` or `-X utf8=0` ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-warnings-deprecated` (SMP524) reports functions and classes that start with a `DeprecationWarning` warning from
  Python 3.13, where `@warnings.deprecated` marks them for type checkers. The fix is unsafe: a class then warns when
  instantiated, not when defined ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-0-changes"></a>

### Changes

- `constructor-after-method` (SMP203) is renamed `class-member-order`, and it now orders the whole class body rather
  than only its constructors. A class body reads in sections: nested classes, class attributes, constructors in the
  order Python runs them, alternative constructors, other dunder methods, properties, then methods. A member that sits
  above a member of an earlier section is reported, with the section it belongs with, and one fix moves the members into
  place. A helper still follows its first caller, whatever its section, and `definition-before-use` (SMP201) and this
  rule compute the same order, so one `--fix` settles a class body. Both count moving a method whose decorators are all
  pure, such as `@property` or `@classmethod`, as a safe fix ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `constant-assert` (SMP804) also reports a `format` call or `%` expression on a string literal whose text lies outside
  its fields, and a computation on literals that is truthy, such as `1 + 1 == 2`. An assertion on a generator
  expression, which is always truthy, now gets an unsafe fix that wraps it in `all()`, unless the generator is
  asynchronous ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-0-fixed"></a>

### Fixed

- `absolute-sibling-import` (SMP302) no longer reports a stale cached result when an `__init__.py` is added to or
  removed from a directory listed in `namespace-packages`, where the file's package stays the same
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- A fix that removes a statement between two blank lines leaves one of them, as `ruff format` would, so files that
  `ruff format` does not run on, such as `# fmt: off` ones, gain no new E303
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- Docstring fixes (SMP901, SMP902) no longer rewrite a file with a file-level `ruff: noqa`, `flake8: noqa` or
  `ruff: file-ignore`. Removing or reflowing the docstring could leave the codes of that directive unused, which Ruff
  then reports as RUF100 ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` (SMP401) removes the `# noqa: F401` it makes unused when it lists the name in `__all__`, so Ruff's
  RUF100 no longer reports it. Names imported from one line share that `noqa`, which is removed once; before, an
  unapplied fix for them crashed simplint while it rendered the report
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) no longer moves a name that a function rebinds through `global`, or that a file of
  the run stores on the module, into an import. The import would keep the value the module had when it loaded, so the
  rebinding would not reach the importer ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) keep the names of a file named after a standard
  library module, such as `typing_extensions.py`, when another file of the run reads them off that module. The import
  resolves to the standard library, so the run cannot tell which file its reads reach
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102), `module-variable-prefix` (SMP404) and `unexported-public-name` (SMP403) no longer
  rename a name that a string of any file in the run spells, such as the `_compat.repr_context` a generated code
  template reads or the `"main"` a test looks up with `runpy`. The check used to cover only the file being fixed, so an
  unsafe fix could break a test in another file ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `blanket-ignore` (SMP004) and `suppression-without-reason` (SMP003) read `noqa` comments as Ruff does. A `# noqa:`
  whose colon names no code, a code that runs into other text, a `# ruff: noqa` after code on its line and a
  `# ruff: file-ignore` below module scope suppress nothing in Ruff, so they are no longer reported. A `noqa` comment
  silences simplint's errors by the same reading ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `test-class` (SMP801) no longer flattens a test class with an E501 `noqa` or `ruff: ignore` in it: dedenting its
  methods shortens the line the directive covers, which Ruff then reported as RUF100
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer inlines a list comprehension whose elements call something into a call to
  `all` or `any`. Ruff's C419 then wrote it as a generator, which stops at the first false element, so the later calls
  never ran ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) decides on the layout a value takes once `ruff format` has run, so a second run after
  `ruff format` no longer inlines more. A value the source spreads over lines is inlined when the formatter would join
  it, into an f-string before Python 3.12 too, a value in parentheses the formatter drops is measured without them, and
  a value assigned before a semicolon is inlined into the statement after it
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer offers a fix before Python 3.12 that puts an f-string with a string in its
  replacement field into another f-string. Such a fix broke the file's syntax, and simplint reverted every fix in it
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) counts a name that a string annotation or `ForwardRef` reads, so it no longer inlines a
  `TypeVar` read only in `"list[T]"`, which left the annotation naming an undefined name
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- Without a `pyproject.toml`, a module under a `test` or `tests` package now counts as a test module wherever it sits in
  that package, so helpers and data files there get the test-only rules, such as `test-module-docstring` (SMP902), and
  their uses count as test uses for rules such as `test-only-code` (SMP408). The settings root is the directory the run
  starts from, as Ruff roots a project without configuration at the working directory, instead of each file's directory
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` (SMP403) no longer renames a function or class whose `__name__` or `__qualname__` the run
  reads, as `helper.__name__` or `type(self).__name__` does, since the rename changes what the read returns
  ([#55](https://github.com/gaborbernat/simplint/pull/55)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-7-0-performance"></a>

### Performance

- The cache keys a file's result by the other files of the run only when an enabled rule reads them, so an edit to one
  file no longer re-lints files checked by per-file rules alone. The reStructuredText rules that need no public API data
  no longer count as reading other files ([#55](https://github.com/gaborbernat/simplint/pull/55)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.6.0 (2026-11-05)

<a name="release-0-6-0"></a>

<a name="release-0-6-0-added"></a>

### Added

- `constructor-after-method` (SMP203) reports a constructor (`__new__`, `__init__`, `__post_init__` or
  `__init_subclass__`) defined below another method of its class, and moves it above that method.
  `definition-before-use` (SMP201) no longer moves a constructor below a method that calls it, so the two fixes agree
  ([#54](https://github.com/gaborbernat/simplint/pull/54)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-6-0-fixed"></a>

### Fixed

- `definition-before-use` (SMP201) reads a method's first parameter as the instance or class whatever its name, so
  `klass._make()` in a classmethod counts as a use, as do `klass().load()` and a local that a method assigns from
  `klass()` or `super().__new__(klass)`. A lambda in a class body, as `property(lambda self: self._load())`, reads its
  first parameter the same way. A `staticmethod` has no such parameter, so `self` or `cls` in one no longer counts as a
  use ([#54](https://github.com/gaborbernat/simplint/pull/54)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.5.1 (2026-11-05)

<a name="release-0-5-1"></a>

<a name="release-0-5-1-fixed"></a>

### Fixed

- `dataclass-frozen` (SMP701) and `dataclass-slots` (SMP702) treat whatever a method names its first parameter as the
  instance, not only `self`, so attributes set through `def m(this)` count
  ([#50](https://github.com/gaborbernat/simplint/pull/50)) by [@gaborbernat](https://github.com/gaborbernat).
- `mock-without-spec` (SMP806) flags `PropertyMock` too, matching the mock classes the project index knows
  ([#50](https://github.com/gaborbernat/simplint/pull/50)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-asyncio-iscoroutinefunction` (SMP516) adds its `inspect` import in the standard-library section, where isort puts
  it, as `up-chdir-restore` (SMP513) and `missing-final` (SMP601) do
  ([#50](https://github.com/gaborbernat/simplint/pull/50)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-5-1-security"></a>

### Security

- `simplint clean` no longer follows a `.ruff_cache` that is a symlink, which could delete a `simplint` directory
  outside the project ([#50](https://github.com/gaborbernat/simplint/pull/50)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-5-1-performance"></a>

### Performance

- Repeated runs without `--fix` reuse the cached reports of files that did not change, rather than only skipping files
  that had none, and the project index resolves each imported module once per directory
  ([#50](https://github.com/gaborbernat/simplint/pull/50)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.5.0 (2026-11-05)

<a name="release-0-5-0"></a>

<a name="release-0-5-0-added"></a>

### Added

- simplint publishes a JSON Schema of the `[tool.simplint]` table as `simplint.schema.json`, which editors use to check
  the table as the user types. `simplint config-schema` prints it
  ([#46](https://github.com/gaborbernat/simplint/pull/46)) by [@gaborbernat](https://github.com/gaborbernat).
- Types in a public signature count as public API: a class or alias that an exported name's annotations or bases refer
  to is no longer reported as unused in `__all__`, and missing-from-all lists it there
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-argparse-suggest-on-error` (SMP519) flags `suggest_on_error=True` in `argparse.ArgumentParser()` calls, which
  Python 3.15 makes the default, with a safe fix that drops the keyword
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-asyncio-iscoroutinefunction` (SMP516) flags `asyncio.iscoroutinefunction()` calls, which
  `inspect.iscoroutinefunction()` replaces, with an unsafe fix since the two differ for legacy coroutine markers
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-backport-import` (SMP305) flags imports of backports the standard library replaced, such as `tomli` for `tomllib`
  on Python 3.11 and later; the `tomli` and `exceptiongroup` imports get a fix
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-chdir-restore` (SMP513) flags a working directory restored in a `finally` clause, which `contextlib.chdir` does on
  Python 3.11 and later, with a safe fix that adds the `contextlib` import
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-codecs-open` (SMP517) flags `codecs.open()` calls, which the builtin `open()` replaces with `newline=""` and an
  encoding, with an unsafe fix since line iteration differs for some separators
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-comprehension-unpacking` (SMP508) flags a nested `for` clause that only unpacks its iterable, which Python 3.15
  lets the element do with `*` or `**`, with a safe fix for lists, sets and generators
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-context-var-token-reset` (SMP512) flags a `ContextVar.set()` token that a `try` block resets in its `finally`
  clause, which a `with` block does on Python 3.14 and later, with a safe fix
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-event-loop-in-coroutine` (SMP515) flags `asyncio.get_event_loop()` calls in coroutines, which
  `asyncio.get_running_loop()` replaces on Python 3.10 and later, with a safe fix
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-exc-info-exception` (SMP510) flags `sys.exc_info()` reads of the exception being handled, which `sys.exception()`
  returns directly on Python 3.11 and later, with a safe fix ([#48](https://github.com/gaborbernat/simplint/pull/48)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-lazy-modules-list` (SMP307) flags a `__lazy_modules__` list whose imports Python 3.15 can spell as `lazy import`,
  and fixes both ([#48](https://github.com/gaborbernat/simplint/pull/48)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-object-sentinel` (SMP511) flags module and class constants assigned a bare `object()`, which the Python 3.15
  `sentinel` builtin names, with an unsafe fix since the repr changes
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-re-error-alias` (SMP518) flags `re.error` references, which `re.PatternError` replaces on Python 3.13 and later,
  with a safe fix ([#48](https://github.com/gaborbernat/simplint/pull/48)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-removed-ast-node` (SMP520) flags `ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis`
  references and `visit_` methods that Python 3.14 removed, with safe fixes for the common `isinstance` and constructor
  forms ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-strptime-date` (SMP514) flags `datetime.strptime(...).date()` and `.time()` calls, which `date.strptime` and
  `time.strptime` replace on Python 3.14 and later, with a safe fix for dates and for times in literal formats without
  `%z` ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-traceback-exception-arguments` (SMP509) flags `traceback` calls that pass an exception's type and traceback next
  to the exception, which Python 3.10 reads from the exception, with a safe fix
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-type-checking-lazy-import` (SMP306) flags `if TYPE_CHECKING:` blocks of imports that Python 3.15 can spell as
  `lazy` imports, with an unsafe fix ([#48](https://github.com/gaborbernat/simplint/pull/48)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `up-unittest-assert-helper` (SMP811) flags `assertTrue` and `assertFalse` on `startswith`, `endswith`, `hasattr` and
  `issubclass` checks in `TestCase` methods, which the Python 3.14 `assertStartsWith` family replaces
  ([#48](https://github.com/gaborbernat/simplint/pull/48)) by [@gaborbernat](https://github.com/gaborbernat).
- `up-unittest-enter-context` (SMP812) flags a patch that a `TestCase` method starts and stops by hand, which
  `self.enterContext` replaces on Python 3.11 and later ([#48](https://github.com/gaborbernat/simplint/pull/48)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-5-0-fixed"></a>

### Fixed

- `constant-assert` (SMP804) no longer offers a fix that writes invalid syntax when the tuple holds a walrus or a
  starred element: a walrus check gets parentheses, and a tuple with a starred element gets no fix
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-kw-only` (SMP703) passes the fields of a call in an f-string `=` debug field by keyword without changing
  the printed text. The call's source text stays, and its value follows in a plain field
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` (SMP201) no longer reads the bound or default of a type parameter, or the value of a `type`
  statement, as a read at import time. A helper that the same function calls is reported again
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `dependency-import-guard` (SMP303) counts a submodule of a standard library package, such as `asyncio.taskgroups`, as
  shipped only from the version that adds it, using a table generated from CPython's `Lib/` trees, so the guard around
  it is reported on the targets that have it. Its unsafe fix removed an import left unused by the unwrap, such as
  `import string.templatelib` checked only for existence, and keeps it now with a `noqa` suppression. The guards around
  `_zstd`, `compression.zstd` and `_curses` stay unreported ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP901) reads the functions in a `match` statement's `case` bodies, at module and class level,
  as it already reads those in `if` and `try` blocks, so a docstring on an internal one is reported
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `future-annotations` (SMP301) no longer reports the import when a class annotation reads a name the class body binds,
  such as a method named `list` in `def list(self) -> list[str]`. Without the import, Python 3.14 and later evaluate
  that annotation in the class namespace, where `list` is the method
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) now also reports `object` and `Any` in the value of a `type` statement and in the
  bounds and defaults of type parameters ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `indistinct-error-class` (SMP704) no longer reports an exception class that a handler selects through
  `ExceptionGroup.split(...)` or `.subgroup(...)` ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `init-module-content` (SMP407) accepts `__lazy_modules__` in a public package's `__init__.py`, which Python 3.15 reads
  from the module globals to make the package's imports lazy ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) keeps the `lazy` keyword when its fix rewrites a lazy import, so the module still
  loads on first use of a name. Its fix is no longer offered when a parameter, local, comprehension target, match
  capture or type parameter of the same name would take the bare name, which made `make[Item]` call a `TypeVar`
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) no longer renames a name read in a t-string, where the kept text would change. The
  name is still reported, without a fix. A name read in an f-string `=` debug field is renamed with the printed text
  unchanged ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) no longer renames a `type` alias. The statement binds a type rather than a value, so
  `type vector = list[float]` keeps its name ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-prefix` (SMP404) no longer renames a name read in a t-string, where the kept text would change. The
  name is still reported, without a fix. A name read in an f-string `=` debug field is renamed with the printed text
  unchanged ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-prefix` (SMP404) no longer asks for a leading underscore on a `type` alias
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `rst-returns` (SMP923) reads a quoted return annotation, such as `-> "None"` or `-> 'NoReturn'`, as the unquoted form
  it spells. Before, a quoted `None` counted as returning a value
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `rst-unknown-attribute` (SMP921) accepts a `:cvar` field that documents a `type` statement in the class body, and no
  longer deletes that field ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-letter-name` (SMP101) now reports a single-letter `**rest` capture in a mapping pattern, as it already does
  for `*rest` in a sequence pattern ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-use-variable` (SMP504) no longer inlines a variable where the target Python version rejects the result or the
  meaning changes: a value with a backslash before Python 3.12, a conditional read, an annotation, a `type` statement,
  an `except` type, a `match` case, or a decorator before Python 3.9. Before Python 3.12, a value whose strings use the
  f-string's quote takes the other quote in those strings. A variable read in an f-string `=` debug field is inlined
  with the printed text unchanged. A value that starts with `{` gets a space inside its field, so the field does not
  read as an escaped brace ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` (SMP202) no longer offers a fix for a constant read in a t-string, where inlining or renaming
  would change the kept text. The constant is still reported. A constant read in an f-string `=` debug field is inlined
  or renamed with the printed text unchanged ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP503) wraps a generator passed bare to `str`, `repr` or `ascii` in parentheses, so its fix
  no longer writes a syntax error. A name bound by a PEP 695 type parameter is no longer treated as a `str`, so its
  chain is reported without a fix ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `test-class` (SMP801) offers a fix for a generic class. Each method keeps the class's type parameters that its
  signature, body or nested `type` statements use, with their bounds and defaults, so `class TestBox[T]` becomes
  `def test_box_default[T](...)`. Defaults need Python 3.13, so below it a method using a defaulted parameter gets no
  fix ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP903) reports a public `type` statement that has no docstring, as it does for other
  public variables. It also reads the public definitions in a `match` statement's `case` bodies, at module and class
  level, as it already reads those in `if` and `try` blocks, so they are no longer skipped
  ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- `unexported-public-name` (SMP403) no longer renames a name read in a t-string, where the kept text would change. The
  name is still reported, without a fix. A name read in an f-string `=` debug field is renamed with the printed text
  unchanged ([#47](https://github.com/gaborbernat/simplint/pull/47)) by [@gaborbernat](https://github.com/gaborbernat).
- A fix that adds syntax the file's target Python version does not support is reverted, and simplint reports it as a bug
  in the fix, as it does for a fix that breaks the syntax ([#47](https://github.com/gaborbernat/simplint/pull/47)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.4.0 (2026-11-04)

<a name="release-0-4-0"></a>

<a name="release-0-4-0-added"></a>

### Added

- `blanket-ignore` (SMP004) flags every suppression comment that names no error code, such as a bare `# noqa`, a
  `# ruff: noqa`, or a `# type: ignore` or `# ty: ignore` without brackets
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint rule`, its JSON output and the documentation mark the rules that read other files of the run, whose results
  another file can change ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-slots` (SMP702) asks for `slots=True` on a dataclass decorator and its fix adds it, unless checked code
  breaks the slotted class ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Telling exceptions apart by the text of their message is reported as `error-message-matching` (SMP507)
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- Flags exception classes that no `except` clause names, so a built-in exception could replace them
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `long-docstring-paragraph` (SMP904) flags a docstring paragraph longer than `max-paragraph-words` words, as
  `long-paragraph` (SMP002) does for comments ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-attribute-import` (SMP304) flags `module.Name` access to a module of the project, which an import of `Name`
  replaces. The new `imports.module-attributes` option leaves the modules it lists alone
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-prefix` (SMP404) flags a module variable whose leading underscore does not match its visibility: a
  name that no other module imports and `__all__` does not list needs one, and a name `__all__` lists must not have one
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `long-paragraph` (SMP002) and the new `long-docstring-paragraph` (SMP904) read their limit from the new
  `max-paragraph-words` option, 100 words by default ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `rst-empty-field` (SMP914) reports `:param:`, `:raises:`, `:returns:`, `:yields:` and `:var:` fields with no
  description, which `rst-field-syntax` reported before. The fixes of the missing-field rules add such fields, so this
  rule can be turned off on its own ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `simplint rules --output-format json` now lists each rule's examples as parsed blocks with their line numbers, and
  each rule's short group title. A rule page whose examples do not parse reports the error in place of its examples
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- SMP505 flags a private helper with one short call site, which inlining into its caller would keep short
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- SMP504 flags a function local that is assigned once and read once, right below the assignment
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `sleep-in-test` (SMP808) flags `time.sleep` and `asyncio.sleep` calls in test modules that wait on a fixed time
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `split-composite-assert` (SMP807) flags consecutive asserts that compare fields of one object one by one, and its
  unsafe fix joins them into one assert over a tuple ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `stdlib-mock` (SMP809) flags `unittest.mock` imports in test modules of projects that depend on pytest-mock
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- Flags module-level functions and classes that only test modules use, so they can move into the tests
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- A parameter default that every call in the run passes explicitly is reported as `unused-default` (SMP506), with an
  unsafe fix that removes it ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-4-0-changes"></a>

### Changes

- missing-final (SMP601) also reports a bare `Final` annotation and fills in its type
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-options` (SMP701) is renamed to `dataclass-frozen` and asks only for `frozen=True`. Its fix no longer adds
  `slots=True`, which `dataclass-slots` (SMP702) now asks for ([#34](https://github.com/gaborbernat/simplint/pull/34))
  by [@gaborbernat](https://github.com/gaborbernat).
- `rst-field-syntax` now reports aliases of field names, such as `:arg:` or `:return:`, and its fix renames them to
  `:param:`, `:raises:`, `:returns:` or `:yields:`. `:var:`, `:ivar:` and `:cvar:` stay distinct
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `imprecise-annotation` (SMP602) reports `object` in any annotation and `Any` where Ruff's ANN401 does not: in variable
  annotations and nested positions such as `dict[str, Any]` ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) checks only the case of module variables, `UPPER_CASE`, and keeps the leading
  underscore a name has. The underscore is checked by the new `module-variable-prefix` (SMP404)
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- Rule codes now follow their group: the first digit names one of ten groups, so a prefix selects a whole group, such as
  `SMP4` for Public API. The groups are Comments (`SMP0`), Naming (`SMP1`), Module layout (`SMP2`), Imports and
  dependencies (`SMP3`), Public API (`SMP4`), Logic (`SMP5`), Typing (`SMP6`), Classes (`SMP7`), Tests (`SMP8`) and
  Docstrings (`SMP9`) ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `rst-field-syntax` no longer reports fields with no description; `rst-empty-field` does
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- The `docstring-style` value and the reStructuredText rule names use `rst`, such as `docstring-style = "rst"` and
  `rst-field-syntax` ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Rules now group by what they check, one group per topic such as Naming, Public API or Docstrings, instead of four
  groups by code range. `simplint linter` lists each group with its rule codes
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-letter-name` (SMP101) now reports `for` loop targets such as `i` in `for i in range(3)`, since a loop body can
  run far from the line that says what the name holds. Names inside comprehensions stay exempt
  ([#35](https://github.com/gaborbernat/simplint/pull/35)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-4-0-fixed"></a>

### Fixed

- A `cache-dir` in `pyproject.toml` that points outside the project is ignored, and the cache goes to Ruff's default
  directory inside it. Stale cache removal now touches only directories of other simplint builds, so a `cache-dir` can
  no longer delete directories outside the project. The command line and `RUFF_CACHE_DIR` still choose any directory
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- Deeply nested expressions, such as a list nested ten thousand levels deep or a `+` chain of that length, no longer
  crash simplint with a stack overflow ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--fix` and `--diff` give the same result on every run; some fixes used to vary between runs because they followed the
  iteration order of randomly seeded hash maps ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- With `--diff` on several files, the diffs print in the same file order on every run, instead of in the order the files
  finished linting ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- When an `extend`ed configuration file outside the project is not valid, simplint reports its path without its text, so
  the first line of an arbitrary file no longer appears in the error
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).
- A fix loop that does not settle stops with a "Failed to converge" error after 100 rounds, as the loop within a round
  already did, instead of running on ([#34](https://github.com/gaborbernat/simplint/pull/34)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A symlink to a file outside the paths being checked is skipped with a warning. `--fix` no longer writes through it,
  and the report no longer shows the text of the file it points at
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-4-0-documentation"></a>

### Documentation

- The README and documentation pages are split into short sections, with tables, admonitions and diagrams of how
  configuration, the cache, the public API and fix rounds resolve
  ([#34](https://github.com/gaborbernat/simplint/pull/34)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.3.0 (2026-11-04)

<a name="release-0-3-0"></a>

<a name="release-0-3-0-added"></a>

### Added

- The docstring rules share the new `SMP3` group, so `select = ["SMP3"]` picks all of them: `function-docstring` is
  SMP300 (was SMP002), `test-module-docstring` SMP301 (was SMP004) and `undocumented-public-api` SMP302 (was SMP116);
  update `select`, `ignore` and `noqa` comments that name the old codes. Docstrings no longer have a length limit:
  `multiline-module-docstring` (SMP003) is gone, and `long-paragraph` (SMP005) checks comments only
  ([#27](https://github.com/gaborbernat/simplint/pull/27)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP302) and `function-docstring` (SMP300) now follow a library's public API as the typing
  spec and PEP 8 define it: modules under a `_`-prefixed path are internal, a public module without `__all__` offers
  every name without a leading underscore, and a name a public module re-exports through `__all__`,
  `from m import name as name` or `import *` needs its docstring where it is defined. A name another module imports
  counts as public only in a public module, where `missing-from-all` (SMP113) lists it in `__all__`, and an application,
  which nothing installs, has no public API at all. The new `project-kind`, `api-packages` and `api-modules` settings
  tune this ([#27](https://github.com/gaborbernat/simplint/pull/27)) by [@gaborbernat](https://github.com/gaborbernat).
- `init-module-content` (SMP109) lets a public package of a library re-export its API from private modules
  ([#27](https://github.com/gaborbernat/simplint/pull/27)) by [@gaborbernat](https://github.com/gaborbernat).
- The new reStructuredText rules, SMP310 to SMP323, check reStructuredText field lists in docstrings when
  `docstring-style = "rest"`: every field is well formed, described with more than its name and separated from the text
  above by a blank line, documents a parameter or attribute that exists, with `:ivar:` or `:cvar:` as fits, once, and
  carries no type, which belongs in annotations; roles keep their backticks and inline literals close; a public function
  or class documents each parameter, attribute, return or yield value and exception it raises itself, and no internal
  name. Their fixes add each missing field with an empty description for you to fill in, and remove or correct fields
  for what no longer exists ([#27](https://github.com/gaborbernat/simplint/pull/27)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `unused-export` (SMP117) removes names from `__all__` that no other module of the run imports, keeping the `__all__`
  of a library's top-level package and of the modules `api-modules` names as written
  ([#27](https://github.com/gaborbernat/simplint/pull/27)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-3-0-fixed"></a>

### Fixed

- `--fix`, `--add-noqa` and `--add-ignore` write a file by renaming a complete copy over it, so a failed write no longer
  leaves a cut-short source, and a file edited during the run keeps the edit. A symlink stays a symlink and the file
  keeps its permissions ([#33](https://github.com/gaborbernat/simplint/pull/33)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A run no longer deletes the new, still empty cache directory of another simplint version running at the same time, as
  pre-commit and the command line can, which made that other run fail
  ([#33](https://github.com/gaborbernat/simplint/pull/33)) by [@gaborbernat](https://github.com/gaborbernat).
- A development build no longer warns that it cannot read the cache an earlier build of the same version wrote, and the
  caches of other builds go away once no run has written to them for 30 days
  ([#27](https://github.com/gaborbernat/simplint/pull/27)) by [@gaborbernat](https://github.com/gaborbernat).
- A file that linted clean is checked again when a name moves between two of the run's sets, as from a name read off a
  module to a base class, instead of reusing its stale cached result
  ([#33](https://github.com/gaborbernat/simplint/pull/33)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-kw-only` (SMP114) rewrites a positional call that a `noqa` comment for another code sits on, or that holds
  `noqa` in a string, instead of leaving the dataclass without `kw_only=True`; only a comment that suppresses SMP114
  keeps the call as it is ([#33](https://github.com/gaborbernat/simplint/pull/33)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP112) no longer slows to a crawl on a function where many names each extend themselves in
  turn, as `text = text + "!"` does; such chains now take time in proportion to their length
  ([#33](https://github.com/gaborbernat/simplint/pull/33)) by [@gaborbernat](https://github.com/gaborbernat).
- `suppression-without-reason` (SMP060) reads suppression comments the way they take effect, so it now also flags
  `# flake8: noqa` and a directive with extra space around its colon, such as `# ruff :  ignore[...]`
  ([#33](https://github.com/gaborbernat/simplint/pull/33)) by [@gaborbernat](https://github.com/gaborbernat).
- `test-class` (SMP200) now fixes a class whose methods have docstrings spanning several lines, as it does once
  `ruff format` has joined them, so one round of fixes settles ([#28](https://github.com/gaborbernat/simplint/pull/28))
  by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` (SMP302) and the reStructuredText attribute rules now see the docstring under a variable
  defined in an `if` or `try` block, so they no longer report it as undocumented
  ([#33](https://github.com/gaborbernat/simplint/pull/33)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-3-0-documentation"></a>

### Documentation

- The changelog page renders again when a release lists no changes
  ([#18](https://github.com/gaborbernat/simplint/pull/18)) by [@gaborbernat](https://github.com/gaborbernat).
- The pre-commit setup uses the `simplint-pre-commit` mirror, which installs the released wheel and so runs on
  pre-commit.ci without a Rust toolchain ([#27](https://github.com/gaborbernat/simplint/pull/27)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The PyPI page shows a short introduction with links to the documentation instead of the full documentation pages
  ([#20](https://github.com/gaborbernat/simplint/pull/20)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.2.2 (2026-11-02)

<a name="release-0-2-2"></a>

<a name="release-0-2-2-fixed"></a>

### Fixed

- Release wheels build again: the profile-guided optimization run no longer stops on the Ruff docstring rules simplint
  refuses to run beside ([#17](https://github.com/gaborbernat/simplint/pull/17)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.2.1 (2026-11-02)

<a name="release-0-2-1"></a>

## 0.2.0 (2026-11-02)

<a name="release-0-2-0"></a>

<a name="release-0-2-0-added"></a>

### Added

- `all-format` follows Ruff's `skip-magic-trailing-comma`: with it set, `ruff format` drops the trailing comma, so the
  rule only asks for a list ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `all-format` requires `__all__` to be a list with a trailing comma, so the formatter keeps one name per line
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Like Ruff, `simplint check` caches which files had nothing to report, and with them the import facts of each file, so
  later runs skip unchanged files. A cached result is reused only while the file, its settings and the names other files
  use from it are unchanged. The cache lives in Ruff's cache directory, `.ruff_cache` unless Ruff's `cache-dir`,
  `--cache-dir` or `RUFF_CACHE_DIR` names another, under `simplint/<version>`, next to Ruff's own files, which it leaves
  alone. It is a compact binary file per package, written only when it changed, and keeps files other runs checked for
  30 days, as Ruff's does. `--no-cache` or `SIMPLINT_NO_CACHE` turns it off, and `simplint clean` removes simplint's
  part of each `.ruff_cache` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `constant-assert` (SMP205) reads truthiness the way Ruff does, so it also reports assertions on non-empty list, set
  and dict literals, f-strings and lambdas, such as `assert [value]`. A tuple holding only starred elements, which may
  be empty, is no longer reported. An assertion that calls something, such as `assert [compute()]`, gets no fix
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `[tool.simplint]` takes `fixable`, `unfixable`, `extend-fixable`, `extend-unfixable`, `extend-safe-fixes`,
  `extend-unsafe-fixes` and `extend-per-file-ignores`, with Ruff's meaning, and a `per-file-ignores` pattern starting
  with `!` applies to every file it does not match ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Every fix carries a title saying what it changes, such as "Annotate `LIMIT` with `Final`", shown as the `help:` line
  of the full output and as the fix message of the JSON, SARIF and GitHub formats, as Ruff does for its own fixes
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `--config`, `--isolated` and the new `--color` flag go before or after the subcommand, as in Ruff. `--config` also
  takes `KEY = VALUE` overrides: a `[tool.simplint]` key, such as `--config "select = ['SMP1']"`, overrides simplint's
  setting, and any other key Ruff's, such as `--config "target-version = 'py312'"`. `FORCE_COLOR` turns colors on as in
  Ruff ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Add the `naming.keep-names` option, a list of module-scope names `module-variable-case` (SMP102) leaves as they are,
  for frameworks simplint does not recognize; `--config "naming.keep-names = [...]"` overrides it
  ([#8](https://github.com/gaborbernat/simplint/pull/8)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint check` now behaves like `ruff check`: it prints reports in any of Ruff's output formats (`full` by default,
  `concise`, `grouped`, `json`, `json-lines`, `junit`, `github`, `gitlab`, `pylint`, `rdjson`, `azure` and `sarif`),
  with Ruff's summary lines and exit codes, and takes Ruff's flags: `--fix-only`, `--show-fixes`, `--statistics`,
  `--exit-zero`, `--exit-non-zero-on-fix`, `--ignore-noqa`, `--output-file`, `--extend-select`, `--stdin-filename` or
  `-`, and the `--quiet`, `--silent` and `--verbose` log levels. A file that does not parse or cannot be read is
  reported as an `invalid-syntax` or `io-error` diagnostic, and the rest of the run goes on
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint check` takes the rest of `ruff check`'s flags: `--watch` checks again whenever a Python or TOML file
  changes, `--add-noqa[=REASON]` and `--add-ignore[=REASON]` write suppression comments for every report, `--show-files`
  and `--show-settings` print the files and settings a check would use, and `--preview`, `--extend-ignore`,
  `--per-file-ignores`, `--extend-per-file-ignores`, `--fixable`, `--unfixable`, `--extend-fixable` and
  `--extend-unfixable` work as in Ruff, with Ruff's conflicts between flags
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Fixes now leave nothing for Ruff to redo when `simplint check --fix` runs between `ruff check --fix` and
  `ruff format`: a Ruff violation a fix introduces, or a Ruff fix it makes possible such as a return type Ruff can now
  infer, gets Ruff's own fix in the same run, and otherwise the fix is withdrawn and the report kept. Layout is left to
  `ruff format`, save a line no formatting can bring under the line length, such as a name or string too long by itself
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint rule --all`, `simplint config [KEY]`, `simplint linter`, `simplint clean`, `simplint version` and
  `simplint generate-shell-completion SHELL` work like Ruff's subcommands of the same names, and `rule`, `config`,
  `linter` and `version` take `--output-format json` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- simplint stops with exit status 2 before linting when a simplint rule and a Ruff rule on for the same file work
  against each other: `function-docstring` with Ruff's missing-docstring rules D101 to D107, `test-module-docstring`
  with D100 or D104 on test modules, `future-annotations` with I002 adding `from __future__ import annotations` on
  Python 3.14 and newer, and `absolute-sibling-import` with TID252 banning every relative import. The error names both
  rules, the configuration files that enable them, the first file affected and how to resolve the clash. Before, the fix
  loop kept the report forever, or gave up after 100 passes ([#4](https://github.com/gaborbernat/simplint/pull/4)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A `# simplint: ignore[SMP010]` comment suppresses the listed rules on its line, as `# ruff: ignore[...]` does for
  Ruff's rules, and `suppression-without-reason` asks for a reason after it
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` joins strings in different quote styles when one delimiter fits them all, joins implicitly
  concatenated literals, puts an expression written over several lines on one, and before Python 3.12 switches the
  quotes of string literals inside an expression that would clash with the f-string's own. It leaves alone a literal
  with a named escape such as `\N{BULLET}`, and inside another f-string's replacement field keeps the quotes as written
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `undocumented-public-api` requires a docstring on every function, class and public method the module exports through
  `__all__`, and a docstring or `#:` comment on every exported variable
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` also takes an assignment joined by `;` to the statements before it on its line, as `ruff format`
  puts it on a line of its own either way ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-2-0-changes"></a>

### Changes

- `single-letter-name` no longer checks anything inside a comprehension or generator expression; single-letter names
  there are fine ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` offers its fix on the first report of each body only: that one fix reorders the whole body,
  and repeating it on every report made the default output slow on files with many misplaced definitions
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- A file that cannot be read, parsed or fixed is reported as an error and left unchanged, and the other files are still
  checked ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` and `all-format` write `__all__` one name per line with a trailing comma, the layout `ruff format`
  gives it, and `missing-from-all` inserts names in sorted order and above a closing `if __name__ == "__main__":` block
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP002) keeps the text of a docstring that may state a contract: its fix turns any docstring
  longer than one line of five words into a `#` comment block in its place, wrapped to Ruff's line and doc line limits,
  instead of deleting it. A one-line docstring of at most five words, such as `"""Return the value."""`, is still
  deleted ([#7](https://github.com/gaborbernat/simplint/pull/7)) by [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` no longer reports docstrings on the public API, the names `__all__` exports and the public
  methods of exported classes, which `undocumented-public-api` now requires
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-final`'s fix is now unsafe, since the added `Final` shows in the module's runtime `__annotations__`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` writes a trailing comma when it creates `__all__`, and creating `__all__` is now an unsafe fix,
  since it narrows what `from module import *` exports ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Module names in imports, attribute access, patch targets and entry points now resolve to files with ty's module
  resolver, the one behind `ruff analyze graph`, searching Ruff's `src` directories and package roots. Cross-file rules
  such as `missing-from-all` and the rename fixes see `src` layouts and pytest rootdir test packages correctly and no
  longer match a module by a guessed dotted suffix of its path ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- simplint no longer repeats Ruff's warnings about Ruff's own configuration, such as a removed rule left in `ignore` or
  a deprecated top-level setting, on every run; `ruff check` already shows them. Warnings about simplint's own work stay
  ([#8](https://github.com/gaborbernat/simplint/pull/8)) by [@gaborbernat](https://github.com/gaborbernat).
- Files and the target Python version now come from Ruff's own configuration resolution: `--config`, `--isolated`,
  `extend` chains, user-level Ruff configuration, Ruff's built-in excludes, `lint.exclude`, `respect-gitignore`,
  `force-exclude` and `per-file-target-version` all apply, and `--target-version`, `--exclude`, `--extend-exclude`,
  `--respect-gitignore` and `--force-exclude` override them. Without any version setting the target is Ruff's default,
  Python 3.14, instead of 3.11. `[tool.simplint]` also takes `fix`, `fix-only`, `unsafe-fixes`, `show-fixes` and
  `output-format`, and rejects unknown keys ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Imports resolve with `ruff_graph`'s module database and Ruff's import helpers, so a file's package follows Ruff's
  package detection, including `namespace-packages`, and a relative import reaching above the top-level package resolves
  to nothing, as in Python ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `simplint rule` prints like `ruff rule`: a rule without a fix no longer says so, and an unknown rule is rejected as an
  invalid argument ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- When `select` and `ignore` both match a rule, the more specific entry wins, as in Ruff:
  `--select SMP001 --ignore SMP0` keeps `SMP001`. Before, `ignore` always won
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `mock-without-spec`, `mock-attribute-assignment`, `parametrize-without-id`, the dataclass rules, `missing-final` and
  `callable-ellipsis` resolve `patch`, `pytest.mark.parametrize`, `pytest.param`, `dataclass`, `TypeVar` and `Callable`
  through the module's imports, as Ruff does, instead of matching the last part of the name
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` (SMP050) reports only constants whose value is written out in the source: strings, bytes,
  numbers, booleans, `None`, operations between them, `frozenset` of them, and tuple, list, set and dict displays of
  them. A call result such as a cache, a logger, a compiled pattern, a path or a client, and a name or attribute read
  from elsewhere, stays at module level, since moving it would build a new object on every call and break tests that
  patch it ([#7](https://github.com/gaborbernat/simplint/pull/7)) by [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` turns `str(x)`, `repr(x)` and `ascii(x)` into `!s`, `!r` and `!a` conversions and offers no fix
  when a comment sits between the operands ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The `test-module-docstring` fix is now unsafe, since a fixture package under `tests` may read its docstring
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-2-0-fixed"></a>

### Fixed

- `absolute-sibling-import` (SMP110) no longer reports an import in a directory without an `__init__.py` that Ruff's
  `namespace-packages` makes a package, such as a pytest `tests` directory, where the relative import would fail
  ([#8](https://github.com/gaborbernat/simplint/pull/8)) by [@gaborbernat](https://github.com/gaborbernat).
- `absolute-sibling-import` (SMP110) no longer reports imports in a module that compares `__name__` with a string, such
  as a script with a `__main__` guard or an execnet `__channelexec__` module, nor imports inside a function passed to
  `remote_exec` or `inspect.getsource`. That code runs outside its package, where the relative import fails
  ([#13](https://github.com/gaborbernat/simplint/pull/13)) by [@gaborbernat](https://github.com/gaborbernat).
- `all-format` writes a multi-line `__all__` whose closing bracket follows the last name one name per line, instead of a
  comma before the bracket that Ruff's COM819 removes again ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `all-not-last` moves a `# fmt: off` / `# fmt: on` pair that fences `__all__` along with it, so `ruff format` does not
  reformat the list and strand a suppression comment that covers several names on one line
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `all-not-last` offers no fix when a later block extends `__all__` with `+=`, which the move would leave without a list
  to extend ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `all-not-last` keeps two blank lines before the moved `__all__` when the file has no final newline
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- A file starting with a byte order mark no longer crashes `definition-before-use` and the rules that look for a comment
  above a statement ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` no longer moves a definition below a class body that reads it, as in
  `Interrupted = Interrupted` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Ruff's cleanup after a fix keeps every rule on and only limits which ones it fixes, so it no longer drops suppression
  comments such as `# ruff: ignore[unused-import]` as unused and then removes the imports they kept for other modules
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `long-paragraph` treats each reST field, such as `:param name:`, as its own paragraph, and `section-comment` no longer
  deletes a prose line that ends in `===` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `constant-assert` keeps an element of `assert (check, message)` that is written over several lines inside parentheses
  when it rewrites the assertion, instead of leaving a syntax error
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` (SMP050) keeps a local name for a string whose one use calls `format`, since Ruff's f-string
  rule (UP032) would treat the inlined literal differently depending on the layout
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-kw-only` (SMP114) no longer breaks positional calls such as `Partition(msg.topic(), msg.partition())`: its
  fix passes the arguments of the calls in the same module by keyword, and reports each positional call in other checked
  files with a safe fix of its own, which the same run applies. The fix of the decorator is not offered while a use
  could still pass fields by position, such as a call unpacking `*args` (`**kwargs` passes by keyword and does not block
  it), a subclass, a star import of the module, or a call in a file that ignores the rule
  ([#9](https://github.com/gaborbernat/simplint/pull/9)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-options` (SMP111) no longer freezes a dataclass when a field name is written on an object simplint cannot
  type, such as `envs[name].active = True`, or when the class has a base class other than `Generic[...]` or a subclass.
  It no longer adds `slots=True` when a subclass has several bases, such as `threading.local`, whose layout conflicts
  ([#9](https://github.com/gaborbernat/simplint/pull/9)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-options` (SMP111) now sees writes on instances of classes defined in a package's `__init__.py`, and no
  longer adds `slots=True` to a class that writes a `ClassVar` through an instance, reaches its `__dict__`, is pickled
  with protocol 0 or 1 while mutable, or is frozen with subclasses that store attributes. It offers no fix for a class
  without fields or one a test creates inside `pytest.raises`. `dataclass-kw-only` (SMP114) treats `module.Name(...)`
  positional calls on a module no import binds as uses it cannot rewrite
  ([#14](https://github.com/gaborbernat/simplint/pull/14)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-options` and `dataclass-kw-only` stay silent for targets below Python 3.10, where those dataclass arguments
  do not exist ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` (SMP020) no longer moves a class below a decorated function or class that names it in an
  annotation, such as a pydantic model in a FastAPI route's return annotation: the decorator reads the annotation while
  the module loads, so after the move it met an unresolved forward reference. A name a function uses only in annotations
  inside its body no longer counts as a helper to place below it ([#9](https://github.com/gaborbernat/simplint/pull/9))
  by [@gaborbernat](https://github.com/gaborbernat).
- `definition-before-use` (SMP020) no longer moves a helper below a lambda that runs while the module loads, such as
  `(lambda: helper())()` or a callback passed to a module-level call, or below a class body that calls one of its own
  functions which reads the helper ([#11](https://github.com/gaborbernat/simplint/pull/11)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP002) drops a pragma comment such as `# noqa: E501` that followed a docstring it replaces with
  `...` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Removing the docstring of a test module that is left with comments only no longer leaves blank lines at the end of the
  file, which `ruff format` keeps and Ruff reports ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP002) leaves alone docstrings code reads at runtime: those holding a `>>>` doctest, those of
  objects decorated by anything but a known decorator such as `property` or `pytest.fixture`, and all of them when the
  run reads `__doc__` or calls `getdoc` ([#16](https://github.com/gaborbernat/simplint/pull/16)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `parametrize-without-id` reports `pytest.param(..., id=None)`, and `mock-without-spec` reports `spec=None` and
  `autospec=False`, since both leave the default in place ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- File-level `# ruff: noqa`, `# flake8: noqa` and `# ruff: file-ignore[...]` comments now suppress simplint rules, as
  they do in Ruff ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- With `--fix`, a fix that changes what other files learn from the project index, such as `mock-attribute-assignment`
  (SMP203) folding `user.uuid = 7` into the mock constructor, now gets the files depending on it linted and fixed again
  in the same run, so `dataclass-options` (SMP111) freezes the dataclass whose `uuid` field the fix stopped assigning
  without a second run ([#5](https://github.com/gaborbernat/simplint/pull/5)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Fixes whose edits interleave, such as renames of names used across a module, apply in the same pass, so a module with
  more than a hundred of them no longer fails to converge ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `simplint check --fix` runs its fixes again until they change nothing, so a fix that Ruff's cleanup makes possible,
  such as joining the f-string Ruff writes for `%` formatting, or one withdrawn only next to another rule's pending
  fixes, no longer waits for a second run ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `forwarding-function` no longer reports a function that calls a method on a computed value, such as
  `return make(name).strip()`, or that calls one of its own parameters
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `forwarding-function` skips wrappers with parameter defaults and functions other code patches by name
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) no longer asks to rename module variables a framework reads by name: the revision
  identifiers of an Alembic migration, the settings of a gunicorn configuration file, and an `app` or `application` that
  builds a WSGI or ASGI application with Flask, FastAPI, Starlette, Django, Quart, Sanic, aiohttp, Litestar, Falcon,
  Bottle or Pyramid ([#8](https://github.com/gaborbernat/simplint/pull/8)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dependency-import-guard` (SMP040) no longer reports `from` imports, which can fail on a module that lacks the name on
  some platform or version, such as `from os import geteuid` ([#10](https://github.com/gaborbernat/simplint/pull/10)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` (SMP002) and `test-module-docstring` (SMP004) fix a docstring that shares its line with the
  signature or the next statement, as in `def run(): """Run."""`, instead of only after `ruff format` splits the line
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `future-annotations` marks its fix unsafe only when the code refers to `TYPE_CHECKING`, not when a comment mentions it
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `dependency-import-guard` no longer reports a guarded import of your own package, such as an optional compiled
  extension, and recognizes `builtins.ImportError` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` and `walrus-candidate` parenthesize an inlined value that starts with a parenthesis without
  being wrapped in one, such as `(RuntimeError,) if old else ()`, keep a tuple's parentheses, and inline a value written
  over several lines on one; `single-user-constant` no longer derives a local name that starts with a digit or is a
  keyword ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- The JSON and JSON Lines outputs fill `noqa_row` for simplint's reports with the line a `noqa` comment belongs on, as
  Ruff does, instead of `null` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Fixes are withdrawn when they make a line no `ruff format` layout can fit in more cases: the first line of a string
  over several lines, a `def` line with empty parentheses, an operand after the operator of a split operation, comments
  moved deeper, code under `# fmt: off` or `# fmt: skip` or in a file `ruff format` excludes, and a pragma comment that
  an added annotation moves. Lines Ruff does not report, such as one ending in a URL, or code after a bracket the
  formatter splits, no longer withdraw a fix ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dependency-import-guard` no longer calls a standard library module such as `tomllib` a required dependency,
  `suppression-without-reason` says to list the codes a blanket suppression silences, and `definition-before-use` says
  which definition to move a misplaced one below ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-final` skips implicit type aliases and names another module assigns, adds `Final` to a `from typing import`
  line in isort order, places a new import in the standard library section, and writes `t.Final` for
  `import typing as t` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-final` treats `Annotated[Final[...], ...]` as already final
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` (SMP113) no longer asks to list in `__all__` a name the module imports and uses itself, such as
  `sys` or `Path`, when another module reaches it through this one. An import the module never uses still counts as a
  re-export ([#5](https://github.com/gaborbernat/simplint/pull/5)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` no longer appends a second `__all__` when the module assigns one inside an `if` or `try`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `mock-attribute-assignment` wraps a bare tuple, `:=` or `yield` value in parentheses when it moves it into the mock
  constructor, so `return_value = "a", {}` no longer turns into two arguments
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `mock-without-spec` accepts `new=None` and `new=False`, `mock-attribute-assignment` also reports annotated
  assignments, and its fix is unsafe for `create_autospec` attributes other than `return_value` and `side_effect`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Names imported through a directory without `__init__.py`, such as `from tests.helpers import mark` with pytest's
  rootdir on `sys.path`, now count as imported, so `module-variable-case`, `unexported-public-name` and
  `single-user-constant` no longer rename or move them ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) and `unexported-public-name` (SMP103) keep the names PLY, PEP 249 and the `typer`
  command read by name, and names read off modules loaded by a computed name, such as a test fixture's
  `importlib.import_module`. Relative imports in namespace packages now count as imports, and `missing-from-all`
  (SMP113) no longer lists a name that would shadow the module itself in its package's `import *`
  ([#15](https://github.com/gaborbernat/simplint/pull/15)) by [@gaborbernat](https://github.com/gaborbernat).
- A nested `pyproject.toml` without a `[tool.simplint]` table no longer hides the settings of a parent directory,
  matching how Ruff skips one without `[tool.ruff]` ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A fix that Ruff's cleanup undoes, such as `absolute-sibling-import` in a project that bans relative imports, no longer
  counts as a fix: the file is left alone and `--diff` and `--exit-non-zero-on-fix` see no change
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- A `noqa` comment on the last line of a multi-line string or a backslash continuation suppresses a report that starts
  on an earlier line of it, as in Ruff, instead of being ignored ([#2](https://github.com/gaborbernat/simplint/pull/2))
  by [@gaborbernat](https://github.com/gaborbernat).
- `single-letter-name` skips the parameters of `@override` methods
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` and `undocumented-public-api` no longer treat a name reached only by a dotted string, such as a
  `mocker.patch` target through a module's `import os`, as an export. Fixes that rename or move names still leave such
  names alone ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- The `--preview` help text and docs no longer promise unstable rules and fixes; simplint has none, and the flag
  switches the output to rule names ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `function-docstring` keeps the docstrings of dunder methods, overrides and property setters of an exported class, and
  `undocumented-public-api` no longer asks for them. It accepts a trailing `#:` comment on a variable and names
  properties as such ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` no longer adds `__all__` to a module that imports it from elsewhere
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Rename fixes skip names that shadow a builtin, as in `zip = zip`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) looks for the name in the text strings hold rather than in their source, so whether
  the rename is offered no longer depends on escapes that `ruff format` rewrites, such as `\N{latin small letter x}`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- The cache now accounts for `skip-magic-trailing-comma`; a relative `--config` path anchors per-file ignores at its
  directory; a per-file-ignore pattern such as `./tests/*.py` matches as in Ruff; an empty rule selector is an error; a
  file simplint cannot read is reported as `io-error` only while Ruff's `E902` is on, and otherwise logged as
  `ruff check` does; and the "No Python files found" warning shows once
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Fixes no longer leave Ruff work that only shows up after `ruff format`: a nested `if` or `with` that a fix lets fit on
  one line gets merged, an import block a fix joins across a `;` gets sorted, and a `# noqa: E501` on a line a fix
  changes stays ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Ruff's cleanup after a fix also takes in the rules its own fixes give new work, such as the import sort after an
  import moves to the top, and a fix that would make `ruff format` split a line carrying a `noqa` or other pragma
  comment is withdrawn, since the split moves the comment off the code it applies to
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Ruff fixes that depend on the line length, such as collapsing nested `if` statements, are judged the same before and
  after `ruff format`, so a second run no longer applies what the first withdrew
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `suppression-without-reason` recognizes `# ruff: ignore[...]`, `# ruff: file-ignore[...]` and
  `# pyrefly: ignore[...]`, and directives written without a space after the colon
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint rule` and the rule selection settings now say when a code or selector names a Ruff rule and point to Ruff,
  instead of rejecting it as unknown ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A file simplint cannot read or write is reported with Ruff's own `io-error` (`E902`) diagnostic, at `1:1` as Ruff puts
  it, and with Ruff's `preview` on, reports and the fix summary name rules as Ruff does, unless
  `output-prefer-rule-codes` is set ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The semantic model behind the rules is now Ruff's own, so names follow Python's scoping more closely: PEP 695 type
  parameters and match captures resolve to their own scope, a star import no longer counts as a binding, and a class
  body that annotates `type` without a value no longer hides the builtin
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint rules --output-format json` names a rule's fix availability as `ruff rule --all --output-format json` does:
  `"Always"`, `"Sometimes"` or `"None"`, no longer lowercase ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The SARIF output links to simplint's repository instead of a non-existent `astral-sh/simplint`, and the rdjson output
  names simplint and its documentation as the source instead of Ruff
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `section-comment` (SMP001) no longer deletes the text of a decorated comment with more than three words, such as
  `#  *** remove return to update cron`: its fix strips the decoration and keeps the text. Dividers, and banners around
  a title of up to three words such as `# === helpers ===`, are still deleted. A pair of divider characters joined to a
  word, such as the underscores of `__init__`, no longer makes a comment a section comment
  ([#8](https://github.com/gaborbernat/simplint/pull/8)) by [@gaborbernat](https://github.com/gaborbernat).
- An unknown rule selector error now names the setting and the file it came from, or the command line, as Ruff does
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant`, `missing-from-all` and `all-format` take the indentation of a statement that follows a `;`
  from its line instead of copying the statements before it, which duplicated them
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- A second run of the documented pipeline no longer changes code: `single-user-constant` counts a comment that
  `ruff format` moved inside the parentheses around a value as trailing it and handles a function body on the `def`
  line, `dependency-import-guard` unwraps a one-line `try: import x` instead of breaking the syntax, and the fixes run
  again on the code Ruff's cleanup leaves ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` no longer reports a constant whose only use is a module-level lambda, which it cannot fix
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` no longer reports module values a function mutates, such as a set of warnings already shown or
  a lock, and offers no fix for a constant with a trailing comment or a value that reads a name the module later
  deletes. Its fix keeps the annotation of a new local, and inlines into an f-string only a plain name or number
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` skips Sphinx `conf.py` settings and `TYPE_CHECKING`, `walrus-candidate` and `missing-final`
  skip `TYPE_CHECKING`, and `unexported-public-name` skips Sphinx's `setup` hook
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- The line length model no longer treats an augmented assignment such as `|=` as an operator `ruff format` moves to the
  start of a line, and now does for the `not` of `not in`, so a fix is no longer kept when the line it makes stays too
  long, or withdrawn when `ruff format` can split the line to fit ([#2](https://github.com/gaborbernat/simplint/pull/2))
  by [@gaborbernat](https://github.com/gaborbernat).
- When withdrawing either of two rules would settle a conflict with Ruff, simplint withdraws the one a second run would
  withdraw again, so a single `simplint check --fix` leaves nothing for the next
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Names a `from module import *` can reach count as used: renames to `_`-names by `module-variable-case` (SMP102),
  `unexported-public-name` (SMP103), `single-user-constant` (SMP050) and other rules leave them alone, a name imported
  from a package counts in the module its `__init__.py` star-imports it from, and `missing-from-all` (SMP113) no longer
  creates an `__all__` that would narrow what `import *` takes. A star import that resolves to no file of the run keeps
  the names of every file named after its module ([#5](https://github.com/gaborbernat/simplint/pull/5)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Source read from standard input is linted as the file `--stdin-filename` names would be: it gets that file's
  `per-file-target-version`, its package, and the directory above its package as an import search root, so entry points
  outside the `src` directories count as uses ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `string-concatenation` (SMP112) offers its fix only when the code shows every operand is a `str`, so it no longer
  turns `+` on an object with its own `__add__` or `__radd__`, such as `Markup`, a path or a `PageRange`, into an
  f-string that formats it instead. It reads that from literals, `str` methods, annotations, loops, the module's own
  functions and classes, and common standard library values such as `os.sep` or `os.path.join`
  ([#12](https://github.com/gaborbernat/simplint/pull/12)) by [@gaborbernat](https://github.com/gaborbernat).
- Names a `.pyi` stub declares count as the public API of the module it describes, so fixes no longer rename them out of
  step with the stub ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `suppression-without-reason` no longer reads words such as `# noqa-ish` as a suppression directive
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` and `missing-final` do not run below Python 3.8, and `dependency-import-guard` treats a module as
  standard library only when every supported version ships it, so guards around `distutils`, `ctypes`, `ssl`, `sqlite3`
  or `tkinter` are no longer reported ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `test-class` offers no fix for a class another class extends or the file refers to
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-case` (SMP102) no longer renames `version`, `version_tuple` and `commit_id` in a module that binds
  `__version__`, the names of the version file setuptools-scm writes
  ([#10](https://github.com/gaborbernat/simplint/pull/10)) by [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` (SMP100) and `missing-final` (SMP101) no longer run in a module with a version gate, a top-level
  `if` on `sys.version_info` that raises or exits, since such a module runs on interpreters that lack `:=` or
  `typing.Final` ([#10](https://github.com/gaborbernat/simplint/pull/10)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` (SMP100) no longer reports an assignment when folding it into the `if` test would make the header
  too wide for one line, which `ruff format` then split over several. The header is measured joined on one line at its
  block's indentation, so the result does not depend on how the code is laid out
  ([#8](https://github.com/gaborbernat/simplint/pull/8)) by [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` (SMP100) now also fixes values continued with a backslash, and when its fix leaves an `if` that
  Ruff's collapsible-if (SIM102) asks to merge, the merge happens in the same run whatever the line length, so a second
  run of the pipeline changes nothing ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `walrus-candidate` no longer deletes a module-level name that `__all__` lists, or a class attribute assigned inside a
  block of a class body ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-2-0-documentation"></a>

### Documentation

- Every color pair in the documentation, including code tokens, badges, form fields, focus rings and diagram nodes, now
  reaches a WCAG contrast of 4.5:1 in the light, dark and automatic themes, with calmer diagram colors and a softer page
  background and text ([#3](https://github.com/gaborbernat/simplint/pull/3)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Pages no longer grow wider than the window when a code block holds a long line, as the configuration page did; the
  line scrolls inside its block, and diagrams fit their column ([#6](https://github.com/gaborbernat/simplint/pull/6)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Redesigned documentation with the project branding, a filterable rules index, side-by-side examples, diagrams and a
  generated command-line reference; `simplint rule` leaves the diagram source out of terminal output
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-2-0-performance"></a>

### Performance

- With `--fix` or `--diff`, the cache also remembers files the fixes leave as they are, together with the rules whose
  fixes simplint held back for them, so a later run reports such a file from a single lint instead of running the whole
  fix loop again. On an already fixed copy of `sympy`, a repeat `--fix --unsafe-fixes` run takes about a quarter of the
  CPU time it did ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `simplint check --fix` runs Ruff's findings checks without the whitespace, blank line and line length rules, whose
  findings it ignores, unless a file's suppression comments may name one of them; the fix loop on `sympy` does about 6%
  less work ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- `simplint check` renders its default `full` output on all cores, so a report with many fix diffs, such as renames that
  span whole modules, prints sooner: on `sympy` the run takes about 40% less wall time
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- Release wheels for x86_64 Linux (glibc and musl), x64 Windows and arm64 macOS are built with profile-guided
  optimization, trained on `simplint check` and `simplint check --fix` over pinned sdists of black, pip and botocore. On
  `sympy` and `django` this runs about 6% fewer instructions in check mode and about 12% fewer with `--fix`
  ([#2](https://github.com/gaborbernat/simplint/pull/2)) by [@gaborbernat](https://github.com/gaborbernat).
- The rules that look at statements and expressions share one walk over each file instead of walking it once each, so
  `simplint check` takes about 7% less time ([#2](https://github.com/gaborbernat/simplint/pull/2)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-2-0-new-contributors"></a>

### New Contributors

- [@gaborbernat](https://github.com/gaborbernat) made their first contribution in
  [#2](https://github.com/gaborbernat/simplint/pull/2)

## 0.1.0 (2026-10-30)

<a name="release-0-1-0"></a>

<a name="release-0-1-0-added"></a>

### Added

- First release: 32 rules for code that is easier to read and review, beyond what Ruff checks, with safe and unsafe
  fixes and a cross-module import index by Bernát Gábor.
