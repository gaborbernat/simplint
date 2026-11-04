# Changelog

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
