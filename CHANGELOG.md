# Changelog

## 0.12.2 (2026-11-13)

<a name="release-0-12-2"></a>

<a name="release-0-12-2-changes"></a>

### Changes

- [`conditional-in-test`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/conditional_in_test.md)
  no longer reports branches on the interpreter or its environment, such as `sys.platform` or `os.environ`, nor branches
  on what a `while` loop or a `for` loop over received values got, since neither can become a parameter
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`meaningless-name`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/naming/meaningless_name.md) no
  longer reports the fields of a dataclass, attrs class, pydantic model, `TypedDict`, `NamedTuple` or msgspec `Struct`,
  whose names the data's schema sets ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-letter-name`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/naming/single_letter_name.md) no
  longer reports the parameters of a lambda, which is a single expression and keeps them next to every use, as a
  comprehension does ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`sleep-in-test`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/sleep_in_test.md) reports only
  sleeps in the body of a test, a fixture or a unittest set-up or tear-down method, and no longer those in test servers,
  test doubles and other helpers ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-12-2-fixed"></a>

### Fixed

- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/unused_export.md) and
  [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/missing_from_all.md) offer no
  fix when code outside the tests reads `__all__` off a module it imports by a computed name, as Pygments' lexer loader
  does, since editing the list changed what that loader loads ([#139](https://github.com/gaborbernat/simplint/pull/139))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/unused_default.md),
  [`default-argument-value`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/default_argument_value.md),
  [`unused-return-value`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/unused_return_value.md) and
  [`test-only-parameter`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/test_only_parameter.md) now
  judge a function under a decorator that passes each call on unchanged, such as `functools.cache`,
  `contextlib.contextmanager` or `typing.override`, instead of skipping it
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`callable-ellipsis`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/typing/callable_ellipsis.md)
  leaves `Callable[..., T]` alone as the bound of a type variable, the typing idiom for a decorator that keeps the
  signature of what it wraps ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`catastrophic-regex`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/catastrophic_regex.md)
  leaves a nested repeat alone when only parts that can match nothing follow it and the call need not reach the end of
  the string, since nothing then backtracks into it ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/layout/class_member_order.md) no
  longer reads a class-level annotation as evaluated when the class body runs while annotations are lazy, which withheld
  the fix that moves a nested class it names ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-kw-only`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/classes/dataclass_kw_only.md)
  leaves `@dataclass(init=False)` alone, as without a generated `__init__` the option changes nothing
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-slots` no longer offers a safe fix when code reaches the `__dict__` or a weak reference of an object of
  unknown class, as `vars(obj)` or `weakref.ref(obj)` in a helper do, and withholds it when `weakref.ref(Release(...))`
  or `vars(Release(...))` reach a new instance ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `dataclass-frozen` and `dataclass-slots` no longer offer a safe fix when code sets an attribute no class of the run
  declares on an object of unknown class, such as `node.extra = 1` on a loop variable, which may be an instance of the
  dataclass ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/layout/definition_before_use.md)
  no longer keeps in place the helpers of a base class or of a function whose default or decorator reads a constant, as
  neither runs its body while the module loads, and lets such a function move as long as the names its statement reads
  stay bound above it ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/layout/definition_before_use.md)
  no longer takes a class body that reads a method, as `__str__ = __repr__` or `property(_get)` do, for one that calls
  it, which kept the method's helpers in place ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/layout/definition_before_use.md)
  no longer pins a class named in the signature of a `pytest.fixture`, `pytest.mark`, `pytest.hookimpl`, `mock.patch` or
  `deprecated` function when annotations are lazy, as none of them evaluates the annotations while the module loads
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/duplicate_code.md) no longer
  compares a module under `_sync` with its `_async` twin from which [unasync](https://github.com/python-trio/unasync)
  generates it ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/early_assignment.md) no
  longer holds an assignment back for a string that keys a subscript or a dict display and spells the name, such as
  `loaded["cases"]`, since such a string is never run as code ([#139](https://github.com/gaborbernat/simplint/pull/139))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`expression-complexity`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/expression_complexity.md)
  counts a chain of calls and subscripts on one receiver once, as `query.filter(a).order_by(b)` and `data["a"]["b"]`
  read left to right instead of nesting ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`init-module-content`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/init_module_content.md)
  leaves a `__version__` alone when setuptools' `attr:`, hatch's or pdm's version `path`, flit, bump-my-version,
  bump2version, tbump or `setup.py` reads it, and no longer reports `from __future__` imports or `__lazy_modules__` in a
  package that is not public ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/single_use_variable.md),
  [`dead-store`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/dead_store.md),
  [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/early_assignment.md) and
  [`walrus-candidate`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/walrus_candidate.md) no
  longer take a variable of the same name that a nested function, lambda or comprehension binds for itself as a read of
  the outer one, so they report the outer variable ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`long-tuple-return`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/long_tuple_return.md) leaves
  alone a method that redefines one a base class of the run defines, whose shape the base sets, and `getstate` of an
  incremental codec ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`module-variable-case`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/naming/module_variable_case.md)
  leaves alone a module-level alias of an imported function, of a lowercase attribute of an imported module, such as a
  PEP 517 hook, and of a method of a module-level instance, which keep the callable's name
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`long-paragraph`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/comments/long_paragraph.md) and
  [`long-docstring-paragraph`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/docstrings/long_docstring_paragraph.md)
  count each list item, field, directive and Google or numpydoc entry as a paragraph of its own, and read an empty `#:`
  line as blank ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`parametrize-without-id`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/parametrize_without_id.md)
  no longer reports bytes rows and rows unpacked into several arguments whose values are all strings, numbers, `True`,
  `False` or `None`, since pytest already names them, such as `test_lower[FALSE-false]`
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`section-comment`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/comments/section_comment.md) leaves
  table rules, lines of a `##` paragraph and a URL or code after `##` alone, so its fix no longer deletes them
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/single_use_variable.md) no
  longer offers a fix that runs a value with calls or lookups after a step it used to precede, such as one of two
  `stack.pop()` operands read in reverse order ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/layout/single_user_constant.md)
  leaves alone a run of three or more adjacent literal constants that different functions read, such as a palette,
  instead of scattering it across its users ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A project without `requires-python` now takes its Python version from Poetry's `python` dependency or the setuptools
  `python_requires` in `setup.cfg` or `setup.py`, so rules no longer offer syntax newer than the project supports
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-module-docstring`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/docstrings/test_module_docstring.md)
  keeps a docstring that carries a copyright or license notice
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-process-state`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/test_process_state.md)
  leaves changes alone that a `patch.dict` decorator or call, `monkeypatch`, an autouse fixture of the module, a `patch`
  class decorator or an inherited base class puts back ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`test-without-assertion`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/test_without_assertion.md)
  follows a local function or a method of the test's class passed to a call as a callback, such as an event handler, and
  counts the checks in its body ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unexported-public-name`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/unexported_public_name.md)
  leaves alone a module that looks its own names up by a computed key, as `globals()["do_" + name]` does, since its fix
  renamed the functions that lookup reaches ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unnamed-regex-group`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/unnamed_regex_group.md)
  leaves the groups of a `findall` or `split` call alone, and those of a compiled pattern only called with them, since
  the code receives those groups by position ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/logic/unused_default.md) leaves alone
  the functions of a module the run reads through `getattr(module, name)` with a computed name or through its `__all__`,
  since its fix broke calls such as `getattr(platformdirs, name)()`
  ([#139](https://github.com/gaborbernat/simplint/pull/139)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/api/unused_definition.md) keeps
  the methods of a mixin, a class listed beside another base that may call them, and a nested function with a dunder
  name such as pytest's `__tracebackhide__` ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-fixture`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/testing/unused_fixture.md) leaves
  alone fixtures that pytest plugins such as anyio and pytest-django request by name, like `anyio_backend` and
  `django_db_setup` ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`vars-call`](https://github.com/gaborbernat/simplint/blob/0.12.2/src/rules/typing/vars_call.md) leaves `vars()` on a
  class object alone, such as `vars(cls)` or `vars(type(obj))`, the way to read a class's own namespace without
  inherited names ([#139](https://github.com/gaborbernat/simplint/pull/139)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.12.1 (2026-11-13)

<a name="release-0-12-1"></a>

<a name="release-0-12-1-changes"></a>

### Changes

- [`comprehension-complexity`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/logic/comprehension_complexity.md)
  allows two `for` clauses and reports from the third, as a pairing of two sequences still reads as one step
  ([#137](https://github.com/gaborbernat/simplint/pull/137)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-12-1-fixed"></a>

### Fixed

- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/layout/definition_before_use.md)
  no longer pins a class named in the annotations of a standard decorator that never evaluates them, such as
  `dataclass`, `property`, `contextmanager` or `functools.cache`, when annotations are lazy, and no longer reads
  `@name.setter` as calling the getter while the class body runs
  ([#137](https://github.com/gaborbernat/simplint/pull/137)) by [@gaborbernat](https://github.com/gaborbernat).
- [`empty-collection-comparison`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/logic/empty_collection_comparison.md)
  leaves `.get(key) == []`, `.get(key, None) == []` and `getattr(obj, name, None) == ()` alone: a missing value gives
  `None`, which a truth test would take for an empty one ([#137](https://github.com/gaborbernat/simplint/pull/137)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`iterator-reuse`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/logic/iterator_reuse.md) reads the
  two sides of `a if test else b` as separate branches, so a generator each side consumes is no longer reported
  ([#137](https://github.com/gaborbernat/simplint/pull/137)) by [@gaborbernat](https://github.com/gaborbernat).
- [`mutable-module-constant`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/typing/mutable_module_constant.md)
  keeps a comment inside a `list[...]` annotation when its fix makes it a `tuple[..., ...]`: it renames the base and
  adds the `, ...` instead of rewriting the whole annotation ([#137](https://github.com/gaborbernat/simplint/pull/137))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`mutable-module-constant`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/typing/mutable_module_constant.md)
  leaves `pytestmark` and the other names pytest reads by name alone: its fix made `pytestmark` a tuple, which pytest
  rejects as a single mark ([#137](https://github.com/gaborbernat/simplint/pull/137)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`too-many-attributes`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/classes/too_many_attributes.md)
  and the rules that tell class attributes from instance ones read `__init_subclass__` and `__class_getitem__` as class
  methods, which Python makes them without a decorator ([#137](https://github.com/gaborbernat/simplint/pull/137)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-dependency`](https://github.com/gaborbernat/simplint/blob/0.12.1/src/rules/imports/unused_dependency.md) no
  longer reports the requirements of a project nested under the tests, which a fixture may list unused on purpose
  ([#137](https://github.com/gaborbernat/simplint/pull/137)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.12.0 (2026-11-13)

<a name="release-0-12-0"></a>

<a name="release-0-12-0-added"></a>

### Added

- [`accumulator-loop`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/accumulator_loop.md) reports
  a loop that only adds to a string, bytes or number started empty or at `0` the line before, and its unsafe fix builds
  the value with `str.join()` or `sum()` ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`ambiguous-operator-chain`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/ambiguous_operator_chain.md)
  reports comparison chains that mix operator kinds, such as `key == value in table` or `a != b != c`, and a bitwise
  operation in the middle of a chain, such as `a > 1 & b < 2`, which binds tighter than the comparisons around it
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`anchored-regex-match`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/anchored_regex_match.md)
  reports `re.match()` with a pattern anchored at its end, and `re.search()` with one anchored at both ends, whose `$`
  lets a trailing newline through; the fix calls `re.fullmatch()` without the anchors.
  [`up-re-prefixmatch`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/up_re_prefixmatch.md) leaves
  those calls to it when both are selected ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`async-sync-dunder`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/async_sync_dunder.md)
  reports `async def` on special methods Python calls without awaiting, such as `__len__`, `__enter__` and a
  non-generator `__aiter__` ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`broken-decorator-stack`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/broken_decorator_stack.md)
  reports `@property` over `@staticmethod` or `@classmethod`, and `@classmethod` over `@property`, which stopped working
  in Python 3.13 ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`cached-generator`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/cached_generator.md) reports
  `functools.cache`, `lru_cache` or `cached_property` on a generator, whose cached generator the first caller exhausts
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`catastrophic-regex`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/catastrophic_regex.md)
  reports a nested repeat such as `(a+)+` that backtracks in exponential time on an input that almost matches
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`comprehension-complexity`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/comprehension_complexity.md)
  reports a comprehension with a second `for` or a second `if` clause, a nested loop or a compound filter that a `for`
  statement or a named predicate states better ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dead-store`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/dead_store.md) reports a value
  assigned to a local that the next assignment in the same block replaces before anything reads it, such as `total = 0`
  right before `total = len(items)`. Its fix removes the assignment when the value is a constant
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`default-argument-value`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/default_argument_value.md)
  reports an argument that passes a parameter of a function in the same module the literal its default already gives, as
  a keyword or as the last positional argument. Such an argument pins the call to today's default and suggests the value
  matters there. The safe fix removes it. A parameter every call passes is left to
  [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/unused_default.md)
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dropped-exception-note`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/dropped_exception_note.md)
  reports an `add_note` call on an exception that is then swallowed or never raised
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`duplicate-condition`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/duplicate_condition.md)
  reports an `elif` test an earlier branch already tried, an operand repeated in one `and` or `or`, and a `case` pattern
  an earlier case already matches, none of which changes what runs
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`empty-collection-comparison`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/empty_collection_comparison.md)
  reports `==` and `!=` comparisons with `[]`, `()` or `{}`, and its unsafe fix turns the test of an `if`, `while` or
  conditional expression into a truth test ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`exception-init-args`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/exception_init_args.md)
  reports exception classes whose `__init__` passes its parent different positional arguments from those it takes, or
  requires a keyword-only one, so `pickle` and `copy` cannot rebuild them
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`expression-complexity`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/expression_complexity.md)
  reports an expression nested deeper than `logic.max-expression-complexity`, 8 by default, where each operator,
  comparison, call, subscript, lambda and comprehension adds a level. Naming the parts spares the reader holding every
  partial result in mind ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`failed-swap`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/failed_swap.md) reports `a = b`
  followed by `b = a`, which leaves both holding the old `b`, with an unsafe fix to `a, b = b, a`, and augmented
  assignments such as `total += total + delta` that apply their target twice
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`function-as-condition`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/function_as_condition.md)
  reports a function tested without being called, as `if is_ready:`, and calls it when it takes no arguments
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`in-place-result`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/in_place_result.md) reports a
  stored, returned, passed or compared result of an in-place method of a list, dict or set, such as
  `ordered = names.sort()`, and of `random.shuffle()`, which is always `None`
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`iterator-reuse`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/iterator_reuse.md) reports a
  `map()`, `zip()`, generator expression or other one-pass iterator read a second time, or read inside a loop, where the
  read finds it empty without an error ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`loop-variable-after-loop`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/loop_variable_after_loop.md)
  reports a `for` loop's variable read after a loop with no `break`, where it holds the last item and is unbound when
  the loop never runs ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`misplaced-arguments`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/misplaced_arguments.md)
  reports a call that passes variables named after parameters into other parameters' slots, leaving deliberate swaps
  alone ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`missing-override`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/missing_override.md)
  reports a method that overrides a method of a base class in the run, across files, without `@typing.override`, on
  Python 3.12 and later. With the decorator a type checker reports the method once the base renames or removes what it
  overrides. The safe fix adds `@override` next to the `def` and imports it from `typing`
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`mutable-module-constant`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/typing/mutable_module_constant.md)
  reports an `UPPER_CASE` or `Final` module constant bound to a list, set or dict that no code changes, and its unsafe
  fix makes it a tuple, a frozenset, or a `frozendict` from Python 3.15 and a `MappingProxyType` before it, retyping its
  annotation to match. The fix stays off when a read in the module depends on the type, such as `==` with a list or `+`
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`range-len-loop`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/range_len_loop.md) reports
  `for i in range(len(seq))` loops that read `seq[i]`, and its unsafe fix loops over `seq` itself when the index serves
  only to read the items ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rejected-operand`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/rejected_operand.md)
  reports operator methods that return a boolean or raise `TypeError` for an operand type they do not handle, with an
  unsafe fix to `return NotImplemented` ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-unraised-exception`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/docstrings/rst_unraised_exception.md)
  reports a `:raises X:` field for an exception the function can no longer raise: no `raise` in its body or in the
  same-module functions and methods it calls names `X` or a subclass, and no operation in them raises it on its own. A
  body that calls code outside the module, or does anything else that can raise an arbitrary exception, is left alone.
  The unsafe fix removes the field ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`shadowed-except-handler`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/shadowed_except_handler.md)
  reports an `except` class that an earlier handler or the same tuple already catches through a base class, with a safe
  fix that drops a redundant tuple entry ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`shared-mutable-repetition`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/shared_mutable_repetition.md)
  reports a list or tuple repeated with `*` whose element is a mutable object, such as `[[0] * n] * m`, when the code
  then changes an element through the name it is stored in, which changes every copy. Its unsafe fix builds the element
  in a comprehension ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`side-effect-expression`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/side_effect_expression.md)
  reports a comprehension used as a statement only for the calls in it, and `a and f()` or `a or f()` used as a
  statement. Its fix writes a `for` loop or an `if` ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`slice-affix-comparison`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/slice_affix_comparison.md)
  reports `s[:n] == "..."` and `s[-n:] == "..."` with a literal of length `n`. When `s` is surely a `str` or `bytes`,
  its unsafe fix calls `startswith()` or `endswith()` instead ([#136](https://github.com/gaborbernat/simplint/pull/136))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`test-process-state`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/test_process_state.md)
  reports a test, fixture or test class method that changes an environment variable, the working directory, `sys.path`,
  `sys.argv`, `sys.modules` or a module attribute by hand, which leaks into the tests that run after it; `monkeypatch`
  makes the same change and undoes it ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`test-return-value`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/test_return_value.md)
  reports a test that returns a value, which pytest ignores, and its unsafe fix turns a returned comparison or boolean
  into the `assert` the author meant ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`test-without-assertion`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/test_without_assertion.md)
  reports a test with no `assert`, no `raise` and no call that can fail it, following the module's own helpers, so it
  passes for any code that does not raise. A test whose name, docstring or comments say it checks for crashes or errors
  is left alone ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`too-many-attributes`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/too_many_attributes.md)
  reports a class whose instances hold more attributes than `classes.max-instance-attributes`, 12 by default, counting
  what its methods set on `self` and a dataclass's fields; related attributes belong in classes of their own
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unawaited-coroutine`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/unawaited_coroutine.md)
  reports a call of an `async def` as a statement of its own, whose body never runs, and adds the missing `await` inside
  a coroutine ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unbound-super`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/classes/unbound_super.md) reports
  zero-argument `super()` in generator expressions, pre-3.12 comprehensions, parameterless lambdas and nested functions,
  class bodies and functions outside a class ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`uncollected-test-module`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/uncollected_test_module.md)
  reports a test module whose file name the `python_files` patterns do not match, such as `tests/check_parser.py`, so
  pytest never runs its tests ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`undeclared-dependency`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/imports/undeclared_dependency.md)
  reports an import of a third-party module that no requirement or extra in `[project]` installs, in the modules that
  run when the package is imported. Such an import works where a development tool or another package installed the
  module and fails for a user who installs only what the project lists. Plugin modules, tests, and guarded, deferred or
  version-dependent imports are left alone ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unformatted-exception-args`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/unformatted_exception_args.md)
  reports a builtin exception raised with a `%` or `{}` message and the values meant to fill it, such as
  `raise ValueError("bad %s", value)`, which shows a tuple instead of the message. Its unsafe fix formats the message
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unreachable-code`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/unreachable_code.md) reports
  statements that never run: those after a `return`, `raise`, `break` or `continue`, after an `if` or `try` whose every
  branch leaves, after a `while True` with no `break`, and under an `if False:` or `while 0:`. Its unsafe fix deletes
  them ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unregistered-marker`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/unregistered_marker.md)
  reports a `pytest.mark` name that the project's pytest configuration does not register under `markers`, so a
  misspelled mark that pytest only warns about is caught before the run. It reads the same configuration files pytest
  does ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-dependency`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/imports/unused_dependency.md)
  reports a requirement in `[project] dependencies` that no file of the run outside the tests imports. Every user
  installs it for nothing, and one the tests alone import belongs in a dependency group. The reports sit at the start of
  the `__init__.py` of the project's first top-level package ([#136](https://github.com/gaborbernat/simplint/pull/136))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`up-pytest-deprecated-api`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/up_pytest_deprecated_api.md)
  reports pytest APIs that pytest 8 and 9 removed or no longer honor: nose-style `setup` and `teardown` hooks, marks on
  fixtures, the `tmpdir`, `tmpdir_factory` and `testdir` fixtures, `msg=` to `pytest.fail`, `pytest.skip` and
  `pytest.exit`, `pytest.raises(message=...)` and `pytest.warns(None)`. It renames `msg` to `reason`, and, as an unsafe
  fix, a nose hook to its pytest name ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`wall-clock-duration`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/wall_clock_duration.md)
  reports durations measured with `time.time()`, which jumps when the system clock is set, and its fix measures them
  with `time.perf_counter()` ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`wrapper-without-wraps`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/wrapper_without_wraps.md)
  reports a decorator wrapper that forwards its arguments without `functools.wraps`, and adds the decorator
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-12-0-changes"></a>

### Changes

- [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/unused_definition.md) also
  reports nested functions their enclosing function never refers to, and instance attributes that methods store on
  `self` but nothing in the run reads ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-12-0-fixed"></a>

### Fixed

- [`all-format`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/all_format.md) keeps reporting a
  tuple `__all__` but no longer offers its fix when a test module mentions `__all__` or another module imports it:
  anyio's suite reads every `__all__` through `getattr` and expects a tuple
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/layout/single_user_constant.md)
  and
  [`module-variable-prefix`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/module_variable_prefix.md)
  count `cast(T, module).name = value` as a write to `module.name`, so they no longer move or rename a variable another
  module sets that way, as fastapi's tests set a tutorial module's file path
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-variable-case`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/naming/module_variable_case.md),
  [`unexported-public-name`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/unexported_public_name.md)
  and
  [`module-variable-prefix`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/module_variable_prefix.md)
  keep their reports without a rename fix in a project built with maturin or setuptools-rust, whose native code imports
  Python names simplint cannot see, as cryptography's Rust code imports `serialize_ssh_public_key`
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/single_use_variable.md) no
  longer reports a name bound to a weak reference or proxy. Inlining `ref = weakref.ref(obj)` into its one use lets the
  reference die at once, so code that counts weak references, as pycparser's tests do, sees none
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-class`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/testing/test_class.md) keeps its report
  without a fix when another module imports the class or a string in the run names it: urllib3 imports `TestSSL` from
  another test module, and anyio's tests match qualified names that contain the class
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/logic/unused_default.md) counts calls
  whose result is dropped when it matches a call to a function the import resolver cannot place, such as a script on
  pytest's `pythonpath`. It removed aiobotocore's `full_history` default that such a call still passed
  ([#136](https://github.com/gaborbernat/simplint/pull/136)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.12.0/src/rules/api/unused_definition.md) treats a
  class that defines `__getnewargs__` or `__getnewargs_ex__` as read whole, as it already did for `__reduce__`: pickle
  rebuilds such objects through those hooks ([#136](https://github.com/gaborbernat/simplint/pull/136)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.11.1 (2026-11-12)

<a name="release-0-11-1"></a>

<a name="release-0-11-1-changes"></a>

### Changes

- [`inline-regex`](https://github.com/gaborbernat/simplint/blob/0.11.1/src/rules/logic/inline_regex.md) no longer stops
  the run as a conflict with Ruff's `unnecessary-regular-expression-compile`, so both can stay on. Ruff reports a method
  called on an inline `re.compile` and has no fix. This rule's fix stores the pattern in a module-level constant, which
  neither rule reports ([#135](https://github.com/gaborbernat/simplint/pull/135)) by
  [@gaborbernat](https://github.com/gaborbernat).

## 0.11.0 (2026-11-12)

<a name="release-0-11-0"></a>

<a name="release-0-11-0-added"></a>

### Added

- [`flattened-package`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/flattened_package.md)
  reports each private module whose name shares a `_<prefix>_` prefix with two or more siblings in its directory, such
  as `_html_url.py`, `_html_parse.py` and `_html_css.py`, which a `_html` package that re-exports their names would
  group. It checks a module again when a sibling joins or leaves its directory
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- [`inline-regex`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/logic/inline_regex.md) reports a
  regular expression compiled inside a function, such as `re.fullmatch(r"...", text)`, and its unsafe fix moves the
  pattern to a module-level constant compiled once. A call that goes through `re` looks its pattern up in a bounded
  cache on every call, and a program that uses more patterns than the cache holds compiles some of them again each time,
  so the constant keeps those calls fast ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/module_attribute_import.md)
  takes an `imports.module-attribute-scope` setting that widens it from the project's own modules to the standard
  library (`"stdlib"`), every other module (`"third-party"`) or one module and its submodules by name, such as
  `"requests"`. The default, `["project"]`, keeps today's behavior. A module outside the run is reported without a fix.
  A module imported under the alias Ruff's `flake8-import-conventions` asks for, such as `numpy` as `np`, one its
  `banned-from` lists, and a name a string patches, such as `mocker.patch("requests.get")`, stay reached through the
  module ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-11-0-changes"></a>

### Changes

- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/api/missing_from_all.md) has a
  Conflicts with Ruff section for the import it reports that another module uses. Ruff's unused-import (F401) deletes
  that import with a safe fix, so running `ruff check --fix` before simplint breaks the importer. The page recommends
  keeping the import with a `# noqa: F401` that gives a reason, or running simplint's fix first
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- The [`mock-without-spec`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/testing/mock_without_spec.md)
  rule now reports a `spec`, `spec_set` or positional spec and a `wraps` argument, since they do not check call
  signatures, and asks for `autospec`. A spec for a class the module defines stays accepted when the test reads an
  instance attribute or a property that autospeccing cannot see through the mock. A `patch(..., spec=True)` gets an
  unsafe fix to `autospec=True`. `autospec`, `new` and `new_callable` count only on `patch` calls: on `Mock(...)` they
  set attributes and check nothing ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The
  [`parametrize-without-id`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/testing/parametrize_without_id.md)
  rule now reports an `ids=` list of string literals, since the list drifts from the rows it names, and offers a fix
  that moves each id into its row as `pytest.param(..., id=...)`. A module-level name bound once to such a list is
  reported without a fix. A callable, or a name the rule cannot read, is left alone
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/private_import_path.md)
  names the fix for each private import it reports: a re-export in the `__init__.py` of the private package, an import
  from a public module, or a public name for a private name ([#130](https://github.com/gaborbernat/simplint/pull/130))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`unnamed-regex-group`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/logic/unnamed_regex_group.md)
  and [`unexplained-regex`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/logic/unexplained_regex.md)
  now also read the patterns of `re.prefixmatch()` calls, the function Python 3.15 adds next to `re.match()`, so a
  prefixmatch pattern with unnamed groups or four unexplained parts is reported like the other `re` functions
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-duplicate-param`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/docstrings/rst_duplicate_param.md)
  follows the docstring Sphinx renders for each class, resolved per project from a `:class-doc-from:` option on its
  `autoclass` or `autoexception` directive, then `autodoc_default_options` for a class such a directive names, then
  `autoclass_content` in the project's Sphinx configuration. A directive names a class by its dotted name, or by a
  simple name that the page's `module` or `currentmodule` qualifies. With no module on the page, a simple name shared by
  two classes of the run leaves both alone. Under `init` the rule reports the copy in the class docstring and removes
  that one, and under `both` it keeps reporting the `__init__` copy. A configuration counts when it sits beside an index
  page or names `extensions`, `master_doc` or `root_doc`, and a wildcard import does not count. A value the rule cannot
  read leaves every copy alone ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/layout/single_user_constant.md)
  no longer reports a list, set or dict display, or a `frozenset` call, since inlining one rebuilds it on every call. It
  reports a binary operation only where CPython folds it at compile time, within CPython 3.11's limits: an integer
  result of 128 bits, a string or bytes repetition of 4096 characters, and a tuple repetition of 256 items, with nested
  items counted up to 1024. A division by zero or a type error is left to run per call
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- The [`test-only-parameter`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/api/test_only_parameter.md)
  rule no longer reports a defaulted parameter that injects a collaborator, such as a class-typed parameter the body
  uses, a `None` default the body calls, or a default that resolves to a function, a class, a module or an instance
  built by calling a class, since a test passes a stand-in for it by design. Data, names the rule cannot resolve, and
  enums stay reported, so a `None` default the body only reads is reported too
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-11-0-fixed"></a>

### Fixed

- [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/classes/dataclass_slots.md) no
  longer reports a dataclass whose body uses zero-argument `super()` or `__class__` when the target version is before
  3.14. Slotting such a class made `super()` raise `TypeError` until CPython fixed it in 3.14 (gh-90562), and 3.13 has
  the fix only from 3.13.14, so 3.13 counts as unfixed ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/module_attribute_import.md)
  keeps its report without a fix when the file and the module it reads from are in one cycle of runtime imports. The fix
  turned `module.Name` reads into a from-import, which fails at load time when the cycle reaches the name before the
  module defines it ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Reject imports that read private names across modules, including direct private children and paired tests. Shared
  names must be public in their owning module and re-exported at each package level. gettext's `_` and imports from
  `_typeshed` stay allowed ([#129](https://github.com/gaborbernat/simplint/pull/129)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/private_import_path.md)
  no longer reports a private import that the code uses at runtime when a string type expression, such as
  `cast("_SubParsersAction", value)`, also names it: Ruff's import-private-name (PLC2701) reports that import, so the
  two no longer both warn ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The docstring rules read reStructuredText the way docutils does.
  [`rst-field-syntax`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/docstrings/rst_field_syntax.md)
  treats a field after a bullet item, literal block or directive line as a new field list, and reports only the first
  field joined to a paragraph.
  [`rst-inline-markup`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/docstrings/rst_inline_markup.md)
  follows docutils' rules for where inline markup starts and ends: a literal closed by a pair followed by a letter is
  unclosed, backticks after a quote mark or inside interpreted text are not, and comments, `raw` and `math` content,
  literal blocks, targets and substitution definitions are not scanned. Its messages name the cause: a Markdown fence, a
  literal followed by a suffix, or backtick runs of different lengths.
  [`rst-unknown-param`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/docstrings/rst_unknown_param.md)
  and the other field rules read escaped colons, colons in names and backticked names as docutils does
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- The reStructuredText and MyST scanners behind the documentation rules now read roles and directives as docutils and
  MyST do. `rst-inline-markup` no longer reports a bare role inside an inline literal, a comment or raw content, and now
  reports one inside quotes or after interpreted text, which docutils reads as a role. `unused-definition` and the other
  rules that read the project's pages count the roles and directives docutils reads: suffix roles, targets and explicit
  titles wrapped across lines, directives after enumerators such as `1.`, `(a)` or `#.`, directives in grid and simple
  table cells, directives in substitution definitions, wrapped options, `{eval-rst}` content, and MyST fences with three
  or more markers, tilde fences and `---` option blocks. A `| text` line outside a grid table is a line block, so it
  holds no directive. Indented code in MyST needs four spaces past the content of the list item it follows, lazy
  continuation lines stay in their list item, and `* * *` or `---` are thematic breaks rather than list items. Roles
  next to non-ASCII punctuation follow docutils' Unicode punctuation classes, which hold ranges of characters, so an em
  dash, an ellipsis, `。` or `．` before or after a role counts as a delimiter, as do `«»`, `‘’` and CJK brackets. Each
  line of a directive's argument is a signature, as Sphinx reads them, and a tab counts to the next multiple of eight
  columns in reStructuredText and four in MyST. MyST fences on a list item's marker line or in a blockquote are read,
  and comments, HTML blocks, autolinks and link destinations hold no roles. The rules no longer count roles in code,
  comments, emphasis or escaped text, or directives in literal blocks and comments
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-11-0-documentation"></a>

### Documentation

- The changelog page of the documentation links a documentation page it mentions, such as the design principles, to that
  page in the documentation instead of its source on GitHub ([#130](https://github.com/gaborbernat/simplint/pull/130))
  by [@gaborbernat](https://github.com/gaborbernat).
- The
  [`entrypoint-module-name`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/naming/entrypoint_module_name.md)
  page recommends keeping a command's code in `__main__.py` and moving it to a separate module only when `__main__.py`
  grows too large ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Rule pages show their Example section as open Flagged and Preferred cards, including pages whose example is a single
  fix, and the examples of Configuration and other sections stay open. Only the examples under What it does fold, and
  the Expand all examples switch covers just those ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The
  [`up-lazy-modules-list`](https://github.com/gaborbernat/simplint/blob/0.11.0/src/rules/imports/up_lazy_modules_list.md)
  page points large codebases adopting lazy imports to [Lifeguard](https://github.com/meta/Lifeguard), which judges
  across a project which modules are safe to load lazily, while the rule rewrites one file's `__lazy_modules__` list
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).
- Rule pages link the standard-library names they mention to the Python documentation, at the addresses its intersphinx
  inventory gives, and the documentation's link check fails when a page names an API it does not link or links an
  address that moved ([#130](https://github.com/gaborbernat/simplint/pull/130)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The changelog links each rule page and project file at the release it describes, so the links in GitHub release notes
  and on PyPI open the file instead of a 404 ([#129](https://github.com/gaborbernat/simplint/pull/129)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Every case a rule page lists under What it does, reported or left alone, now shows an example, and every `up-` rule
  links the PEP or the standard-library documentation that introduced the feature it rewrites to
  ([#130](https://github.com/gaborbernat/simplint/pull/130)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.10.3 (2026-11-11)

<a name="release-0-10-3"></a>

<a name="release-0-10-3-changes"></a>

### Changes

- simplint no longer reports what a Ruff rule reports, preview rules included, whether or not the project's Ruff
  configuration selects that rule; before, a rule left a case to Ruff only while Ruff ran the rule. Rules that already
  left cases to Ruff now always do, and these rules leave Ruff's cases for the first time:
  [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/imports/private_import_path.md)
  (PLC2701),
  [`init-module-content`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/api/init_module_content.md)
  (F401),
  [`up-backport-import`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/imports/up_backport_import.md)
  (A004, when it has no fix),
  [`stdlib-mock`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/testing/stdlib_mock.md) (TID251, when
  `banned-api` bans the import),
  [`section-comment`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/comments/section_comment.md) (E265
  and E266, for a heading written as prose),
  [`function-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/docstrings/function_docstring.md)
  and
  [`test-module-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/docstrings/test_module_docstring.md)
  (D418 and D419, when they have no fix) and
  [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/logic/unused_default.md) (PT028, when
  it has no fix). Select the Ruff rules to have these cases reported.
  [`undocumented-public-api`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/docstrings/undocumented_public_api.md)
  still leaves a case to Ruff's D1 rules only while they run, since the recommended setup turns them off.
  [`all-format`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/api/all_format.md) and
  [`string-concatenation`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/logic/string_concatenation.md)
  also still leave a COM812 or ISC003 case to Ruff only while those run, since `ruff format` moves the line breaks the
  two rules read ([#126](https://github.com/gaborbernat/simplint/pull/126)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`constant-assert`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/testing/constant_assert.md),
  [`mock-assertion-typo`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/testing/mock_assertion_typo.md)
  and
  [`module-variable-case`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/naming/module_variable_case.md)
  report again the cases they can fix that Ruff's F631, PLW0129, PGH005 and N816 report without a fix, so the fix stays
  available ([#126](https://github.com/gaborbernat/simplint/pull/126)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A file that does not parse is skipped with a warning instead of reporting its syntax errors as `invalid-syntax`, which
  Ruff already reports. A run whose only problem is such a file exits with 0
  ([#126](https://github.com/gaborbernat/simplint/pull/126)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-3-fixed"></a>

### Fixed

- [`blanket-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/comments/blanket_ignore.md) no longer
  reports `# type: ignore  [code]`, with spaces before the codes, as blanket; mypy and Ruff read the codes
  ([#126](https://github.com/gaborbernat/simplint/pull/126)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/logic/single_use_variable.md)
  reports `x = ...; return x` in a generator again: it left the case to Ruff's RET504, which skips generators, so
  neither tool reported it ([#126](https://github.com/gaborbernat/simplint/pull/126)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`undocumented-public-api`](https://github.com/gaborbernat/simplint/blob/0.10.3/src/rules/docstrings/undocumented_public_api.md)
  reports a function whose decorator pydocstyle's `ignore-decorators` lists: it left the function to Ruff's D103, which
  skips it ([#126](https://github.com/gaborbernat/simplint/pull/126)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-3-documentation"></a>

### Documentation

- A [design principles](https://github.com/gaborbernat/simplint/blob/0.10.3/docs/explanation/design-principles.rst) page
  explains what simplint reports, when it fixes, and how it splits the work with Ruff
  ([#126](https://github.com/gaborbernat/simplint/pull/126)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.10.2 (2026-11-11)

<a name="release-0-10-2"></a>

<a name="release-0-10-2-changes"></a>

### Changes

- A rule page is now served at `rules/SMP403.html` and a group page at `groups/docstrings.html`, both under the
  documentation's root. The rule links that `simplint rule` and the JSON, SARIF and rdjson outputs print point at these
  pages ([#105](https://github.com/gaborbernat/simplint/pull/105)) by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-2-fixed"></a>

### Fixed

- `--add-ignore` writes its `# simplint: ignore[...]` comment on a line with a blanket `# noqa` too, so the
  [`blanket-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/comments/blanket_ignore.md) and
  [`suppression-without-reason`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/comments/suppression_without_reason.md)
  reports on that line stop showing ([#108](https://github.com/gaborbernat/simplint/pull/108)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--add-noqa=REASON` and `--add-ignore=REASON` replace the whole reason a comment has, including an issue number such
  as `#12`, instead of keeping the text from that `#` on ([#93](https://github.com/gaborbernat/simplint/pull/93)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-field-syntax`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_field_syntax.md)
  (SMP910) renames a field alias such as `:arg:` only with `--unsafe-fixes`, as its other fixes do. A plain `--fix`
  changed the text `__doc__` returns ([#116](https://github.com/gaborbernat/simplint/pull/116)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`all-format`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/all_format.md) (SMP405) leaves a `+=`
  on `__all__` alone when a plain assignment keeps `__all__` a tuple: one that is not a list or tuple literal, is nested
  in a block, or has a `noqa` for this rule. Converting the `+=` used to fail with a `TypeError`
  ([#113](https://github.com/gaborbernat/simplint/pull/113)) by [@gaborbernat](https://github.com/gaborbernat).
- [`all-format`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/all_format.md) (SMP405) puts the
  trailing comma after the closing parenthesis of a parenthesized last name instead of inside it
  ([#113](https://github.com/gaborbernat/simplint/pull/113)) by [@gaborbernat](https://github.com/gaborbernat).
- [`all-not-last`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/all_not_last.md) no longer moves
  `__all__` past a later run-time read of it, such as `sys.modules[__name__].__all__` or `globals()`. The report stays,
  but the fix is withheld ([#91](https://github.com/gaborbernat/simplint/pull/91)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`all-not-last`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/all_not_last.md) no longer moves
  `__all__` when a function or lambda reads it at run time, such as through `globals()`, `"__all__"` or
  `sys.modules[__name__].__all__`, or when a comment sits above it or above its `# fmt: off` pair. The report stays, but
  the fix is withheld ([#113](https://github.com/gaborbernat/simplint/pull/113)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`imprecise-annotation`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/imprecise_annotation.md)
  no longer reports `object` used as a value in the metadata of an `Annotated` annotation, such as
  `Annotated[int, object()]`; `object` as the first argument is still reported
  ([#114](https://github.com/gaborbernat/simplint/pull/114)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-unknown-param`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_unknown_param.md)
  (SMP917) no longer reports `:param x:` of an attrs field `_x`, which attrs takes as `x`.
  [`rst-missing-param`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_missing_param.md)
  (SMP916) asks for `x` in that case, and takes the `alias` of a field as its name. An attrs class with an `alias` that
  is not a string literal has no known parameters, so neither rule reports on it
  ([#116](https://github.com/gaborbernat/simplint/pull/116)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-backport-import`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/up_backport_import.md) no
  longer deletes an `exceptiongroup` import of a name that another module of the run imports from the same module or
  `__all__` lists. It still reports the import ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_context_var_token_reset.md)
  and [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md)
  report without a fix when a backslash joins the assignment to the line before it, instead of producing a syntax error
  that reverts the file ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md) reports
  without a fix when the assignment shares its line with an earlier statement. The fix used to leave that statement on
  the `with` line, which does not parse ([#98](https://github.com/gaborbernat/simplint/pull/98)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`function-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/function_docstring.md)
  (SMP901) keeps the docstring of a class whose body, or the header of a method, lambda or nested class in it, reads
  `__doc__`, which is that docstring there. Removing it changed the value the class reads
  ([#116](https://github.com/gaborbernat/simplint/pull/116)) by [@gaborbernat](https://github.com/gaborbernat).
- [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/class_member_order.md)
  keeps a statement that binds a method's name through a `match` capture or an `except ... as` in place, so the fix no
  longer moves the method past it ([#110](https://github.com/gaborbernat/simplint/pull/110)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-codecs-open`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_codecs_open.md)
  parenthesizes a walrus or a `yield` passed as `encoding`, `errors` or `buffering`, so
  `codecs.open(path, "r", name := "utf-8")` no longer becomes an invalid `open()` call that reverts the file
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-unknown-attribute`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_unknown_attribute.md)
  (SMP921) counts the methods in a class's `if` and `try` blocks and the loop variables of its body as attributes, so
  their fields are no longer removed. A loop variable of the class body is a class variable for
  [`rst-missing-attribute`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_missing_attribute.md)
  (SMP920) and
  [`rst-attribute-kind`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_attribute_kind.md)
  (SMP922) too: SMP920 asks to document it, and SMP922 reports an `:ivar:` for it
  ([#116](https://github.com/gaborbernat/simplint/pull/116)) by [@gaborbernat](https://github.com/gaborbernat).
- [`constant-assert`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/constant_assert.md) (SMP804)
  no longer drops a comment inside an assertion when it rewrites or deletes the assertion. An assertion holding a
  comment is reported without a fix ([#93](https://github.com/gaborbernat/simplint/pull/93)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_context_var_token_reset.md)
  reports without a fix when the assignment shares its line with an earlier statement. The fix used to leave that
  statement on the `with` line, which does not parse ([#98](https://github.com/gaborbernat/simplint/pull/98)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-event-loop-in-coroutine`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_event_loop_in_coroutine.md)
  no longer rewrites a decorator, default or annotation of an `async def`, which runs outside any loop, so
  `async def serve(loop=asyncio.get_running_loop())` no longer raises at import. The same defaults of a nested `def`
  inside a coroutine are now reported, and so are its annotations unless annotations are lazy
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- Cover receiverless classmethods and suppression commands that encounter conflicting rules
  ([#107](https://github.com/gaborbernat/simplint/pull/107)) by [@gaborbernat](https://github.com/gaborbernat).
- CRLF files get the same reports and fixes as LF files. A comment above a constant still exempts it from
  [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/single_user_constant.md),
  and
  [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  moves a comment above a helper along with the helper ([#92](https://github.com/gaborbernat/simplint/pull/92)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Reports that name paths relative to the working directory, from
  [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md),
  [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md) and
  [`undocumented-public-api`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/undocumented_public_api.md),
  are no longer reused by a run from another directory, so they name the paths `--no-cache` does
  ([#108](https://github.com/gaborbernat/simplint/pull/108)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) and
  [`dataclass-kw-only`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_kw_only.md) no
  longer offer a fix for a class whose subclass reaches it under another name, as `from models import Release as Base`
  followed by `class Tagged(Base)`, or through a module-level `Base = Release`
  ([#117](https://github.com/gaborbernat/simplint/pull/117)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) no
  longer offers `frozen=True` for a class written through a parameter annotated `Any`, a protocol, a type alias or a
  type variable, since none of those names a class of the run. A field the class body annotates with a class of the run
  is an instance of that class in its methods, even when no method assigns it, so
  [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) sees the
  writes through it too ([#117](https://github.com/gaborbernat/simplint/pull/117)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) no
  longer offers `slots=True` for a class whose field default is read or assigned through the class, as `Config.timeout`,
  through an import alias, a subclass or `cls` in a classmethod, or through `type(obj)`, `getattr` and `setattr`
  simplint cannot tie to one class, since slots replace that class attribute with a descriptor
  ([#117](https://github.com/gaborbernat/simplint/pull/117)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) and
  [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) no
  longer offer a fix that breaks code setting attributes under a computed name, through `monkeypatch.setattr` or
  `mock.patch.object`, weakly referencing the instance, or decorating the class from below `@dataclass`. They also see
  writes to instances of private classes, such as `_Result`, and of all-caps classes, such as `URL`
  ([#90](https://github.com/gaborbernat/simplint/pull/90)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-kw-only`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_kw_only.md)
  rewrites a positional call inside an annotation, as in `Annotated[str, Doc("...")]` or a string annotation, and writes
  a positional walrus as `name=(name := value)`, where the fix used to make the file invalid Python. A class named in
  the metadata of `Annotated[...]` is a value, so it gets no fix, and neither does one a concatenated string annotation
  names ([#117](https://github.com/gaborbernat/simplint/pull/117)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) makes
  its fix unsafe for a class with a keyword such as `metaclass=`, since the slotted copy runs the metaclass twice
  ([#117](https://github.com/gaborbernat/simplint/pull/117)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) and
  [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) now see
  an instance built as `models.Release(...)`, or annotated `models.Release`, as an instance of the class in that module,
  so a write to it withholds the fix ([#117](https://github.com/gaborbernat/simplint/pull/117)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) and
  [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) keep
  their fixes for a class whose nested class writes to its own instances, or whose classmethod writes a class attribute
  that is not a field. A closure or a parameter that writes the dataclass's instance from a nested class still makes the
  fix unsafe ([#117](https://github.com/gaborbernat/simplint/pull/117)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-slots`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_slots.md) no
  longer offers `slots=True` for an instance that a `weakref.WeakSet`, `WeakKeyDictionary`, `WeakValueDictionary` or
  `WeakMethod` holds, as those hold their items through `weakref.ref`, wherever the container is used in the checked
  files ([#117](https://github.com/gaborbernat/simplint/pull/117)) by [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  and [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/class_member_order.md)
  no longer move a comment indented deeper than the definition below it, such as a trailing note at the end of the
  previous method, along with that definition ([#110](https://github.com/gaborbernat/simplint/pull/110)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  keeps a helper in place when a `def` inside a module-level block, such as `if __name__ == "__main__":`, calls it while
  the module loads, and when a definition reads an import written below it. The fix used to move those helpers above the
  code that needs them, which broke the module at import time ([#91](https://github.com/gaborbernat/simplint/pull/91))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`dependency-import-guard`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/dependency_import_guard.md)
  no longer reports guards around `asyncio.unix_events`, `multiprocessing.popen_forkserver` or the C accelerators
  CPython imports under a fallback, such as `_json`, which a platform or build may lack. Guards around other private
  modules, such as `_thread`, are still reported ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`dependency-import-guard`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/dependency_import_guard.md)
  no longer reports guards around platform-only and optional standard library modules, such as `_overlapped` or `mmap`,
  or around CPython's `_test*` extensions. Its unsafe fix no longer adds a suppression to a function-local import that
  the function uses ([#94](https://github.com/gaborbernat/simplint/pull/94)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-comprehension-unpacking`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_comprehension_unpacking.md)
  removes the parentheses around a dict's key and value too, so
  `{(key): (value) for entry in pairs for key, value in entry.items()}` becomes `{**entry for entry in pairs}` instead
  of an invalid expression that reverted the whole file ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A `--diff` run no longer caches the reports of a file linted after another file's fix, so the next `check` no longer
  loses a [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md)
  report ([#108](https://github.com/gaborbernat/simplint/pull/108)) by [@gaborbernat](https://github.com/gaborbernat).
- `--diff` runs the fix rounds that `--fix` runs, so its diff shows the changes a later round makes, such as a
  [`dataclass-frozen`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_frozen.md) fix
  that becomes safe once a rename leaves the class with no other user. Applying the diff gives the files `--fix` writes
  ([#100](https://github.com/gaborbernat/simplint/pull/100)) by [@gaborbernat](https://github.com/gaborbernat).
- [`function-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/function_docstring.md)
  (SMP901) and
  [`test-module-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/test_module_docstring.md)
  (SMP902) keep a comment after a removed docstring on its own line, which before their fix deleted it along with the
  docstring. A tool directive such as `# fmt: off`, or a comment holding one, goes with the docstring, as a pragma does;
  before, it stayed on its own line and changed what the formatter does
  ([#116](https://github.com/gaborbernat/simplint/pull/116)) by [@gaborbernat](https://github.com/gaborbernat).
- [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md) no longer
  reports statements that repeat under another `if` as a copy, since the blocks they nest in differ; indentation width
  still does not count ([#101](https://github.com/gaborbernat/simplint/pull/101)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md) (SMP530)
  names the first three other modules in sorted order, one place each, and counts the rest. A module that many others
  repeat a block with no longer hides the modules after the first ones behind a single module's copies, so the report
  names a module at the line where its copy starts ([#118](https://github.com/gaborbernat/simplint/pull/118)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/early_assignment.md) (SMP526)
  still reports an assignment with a comment on the line above it, but no longer moves it. The move left the comment
  behind, above the statement that now came first ([#111](https://github.com/gaborbernat/simplint/pull/111)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--fix` keeps the
  [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md) and
  [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md) reports of a
  file it rewrote. Before, the rewritten file lost them, and the run could exit 0 with the cycle still there
  ([#100](https://github.com/gaborbernat/simplint/pull/100)) by [@gaborbernat](https://github.com/gaborbernat).
- [`forwarding-function`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/forwarding_function.md)
  (SMP501) no longer reports a method of a class that another class derives from, since a subclass may override it and
  calling the target at each call site would skip that override. A method that forwards through its receiver, such as
  `self.client.get`, is now reported, and a method that only calls its receiver, such as `self(name)`, is not
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`forwarding-function`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/forwarding_function.md)
  (SMP501) and
  [`single-caller-helper`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_caller_helper.md)
  (SMP505) now count a call whose `(` starts a later line, as in `return [_fill_a` and `(name)]`, the same as a call on
  one line. A name that only ends its statement, such as `callback = _make_path`, is still only read, not called
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`future-annotations`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/future_annotations.md)
  marks its fix unsafe when an annotation unions a string with a type, such as `"Node" | None`, or when an import sits
  under a module-level `if`, since the annotations then fail once evaluated. It no longer offers a fix that would delete
  a comment on the import's lines ([#94](https://github.com/gaborbernat/simplint/pull/94)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`indistinct-error-class`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/indistinct_error_class.md)
  no longer reports a class that an `except` clause catches through a tuple of classes, annotated or starred and defined
  in any module, or that `contextlib.suppress` is given, starred or not
  ([#114](https://github.com/gaborbernat/simplint/pull/114)) by [@gaborbernat](https://github.com/gaborbernat).
- Hard links to one file are linted once, as symlinks already were. `--fix` no longer fails with an E902
  `changed while simplint fixed it` error on them, and their reports no longer repeat once per name
  ([#108](https://github.com/gaborbernat/simplint/pull/108)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md),
  [`up-asyncio-iscoroutinefunction`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_asyncio_iscoroutinefunction.md)
  and [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) put the
  import they add above the comment lines that describe the statement it precedes, instead of between that comment and
  the statement. A line of a docstring that starts with `#` is not such a comment, so the import stays out of the
  docstring ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) no longer
  adds its import above a module docstring that shares its line with the next statement, which turned `__doc__` into
  `None`. The report stays, without the fix ([#92](https://github.com/gaborbernat/simplint/pull/92)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-asyncio-iscoroutinefunction`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_asyncio_iscoroutinefunction.md),
  [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md) and
  [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) keep their
  report without a fix when the last import shares its line with code, since the new import used to land below that
  code, which then ran before it ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md) (SMP308) no
  longer reports the import in the `else` of `sys.version_info[0] == 3`, which no supported Python runs. It counts the
  import under a `TYPE_CHECKING` bound to a true constant, and under a public false constant that a star import rebinds,
  which the module runs and so closes the cycle ([#118](https://github.com/gaborbernat/simplint/pull/118)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md) no longer
  treats a module-level flag as false when a block of the module or a `global` declaration rebinds it, as
  `HAS_JSON = True` in a `try` after `HAS_JSON = False` does, so the imports under the flag count in the cycle check
  ([#101](https://github.com/gaborbernat/simplint/pull/101)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/module_attribute_import.md)
  and
  [`dependency-import-guard`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/dependency_import_guard.md)
  no longer delete a comment inside the import or guard they replace. They still report it, without a fix
  ([#114](https://github.com/gaborbernat/simplint/pull/114)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-inline-markup`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_inline_markup.md)
  checks the rest of a docstring when its summary line ends in `::`, instead of counting every later line as a literal
  block ([#99](https://github.com/gaborbernat/simplint/pull/99)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-argparse-suggest-on-error`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_argparse_suggest_on_error.md)
  and
  [`up-utf8-encoding-default`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_utf8_encoding_default.md)
  keep the parentheses around a positional argument that precedes a removed keyword, so
  `ArgumentParser(("prog"), suggest_on_error=True)` no longer becomes invalid code
  ([#92](https://github.com/gaborbernat/simplint/pull/92)) by [@gaborbernat](https://github.com/gaborbernat).
- [`dataclass-kw-only`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_kw_only.md) no
  longer rewrites `cls(...)` in an ordinary method, where `cls` is some other value, and offers no fix for a dataclass
  another file calls through `getattr(module, "Name")` ([#90](https://github.com/gaborbernat/simplint/pull/90)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-compression-modules`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_compression_modules.md)
  keeps the `lazy` keyword of a `lazy import gzip`, so the rewritten `lazy from compression import gzip` stays lazy
  instead of becoming an eager import ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-lazy-modules-list`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/up_lazy_modules_list.md)
  keeps `__lazy_modules__` when a listed module is also imported under an `if`, `with`, `for`, `while` or `match`, in
  the `else` or `finally` of a `try`, or by a relative import of the listed package, where the fix would have left the
  import eager. An import that is `lazy` already no longer gets a second `lazy`, which produced a syntax error and
  reverted the fixes of the file ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`long-tuple-return`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/long_tuple_return.md)
  (SMP528) no longer reports a function whose return annotation is a quoted `"tuple[T, ...]"`. The quoted annotation is
  read like the unquoted one, which the rule already exempts ([#111](https://github.com/gaborbernat/simplint/pull/111))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) no longer
  suggests `Final` for a module name that a function writes through `globals()`, or for any name when a computed write
  through `globals()` may set one ([#99](https://github.com/gaborbernat/simplint/pull/99)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) no longer
  offers a fix that makes a parenthesized name such as `(LIMIT) = 3` invalid Python. The other names of the module keep
  their fixes ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`missing-final`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/typing/missing_final.md) no longer
  reports a name that a module-level `vars()` or `locals()` write reassigns, including one in the default, decorator or
  base of a `def` or `class` ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md) (SMP401)
  no longer drops a comment when it lists names after the last element of `__all__`, or in an empty `__all__` that holds
  a comment, where it offers no fix. It keeps a `# noqa: F401` on an import that binds a name the listing leaves out
  ([#93](https://github.com/gaborbernat/simplint/pull/93)) by [@gaborbernat](https://github.com/gaborbernat).
- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md) (SMP401)
  adds a new name after the closing parenthesis of a parenthesized last name instead of inside it. The `__all__` no
  longer fails on `from module import *` ([#113](https://github.com/gaborbernat/simplint/pull/113)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `check` reports a path that does not exist as Ruff does, as an `E902` io-error, and exits 1, instead of printing a
  warning and exiting 0 ([#100](https://github.com/gaborbernat/simplint/pull/100)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`mock-assertion-typo`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/mock_assertion_typo.md)
  (SMP814) no longer drops a comment inside an assertion when it renames the call. An assertion holding a comment in the
  text the fix removes is reported without a fix ([#109](https://github.com/gaborbernat/simplint/pull/109)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`mock-attribute-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/mock_attribute_assignment.md)
  (SMP805) no longer passes `cls`, `self` or the private constructor arguments, such as `_new_name`, as keywords. The
  fix used to make `cls` and `self` raise `TypeError` and configure the mock for the others, instead of setting the
  attribute; the run of assignments now stops at them ([#109](https://github.com/gaborbernat/simplint/pull/109)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`mock-attribute-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/mock_attribute_assignment.md)
  offers its fix when a comment follows the trailing comma of the constructor call. The fix used to add a second comma
  before the new keyword, which does not parse, so the fix was rejected
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`mock-assertion-typo`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/mock_assertion_typo.md)
  fixes `assert (mock.called_once_with(1))` to `mock.assert_called_once_with(1)`. The fix used to keep the closing
  parenthesis, which does not parse, so the fix was rejected ([#98](https://github.com/gaborbernat/simplint/pull/98)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-comprehension-unpacking`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_comprehension_unpacking.md),
  [`up-traceback-exception-arguments`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_traceback_exception_arguments.md),
  [`up-exc-info-exception`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_exc_info_exception.md)
  and [`up-strptime-date`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_strptime_date.md) no
  longer delete a comment in the code they rewrite. They report the code without a fix when a comment lies in it
  ([#93](https://github.com/gaborbernat/simplint/pull/93)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/module_attribute_import.md)
  offers no fix that removes a module name another module imports or reads. It also offers no fix for a Python builtin's
  name, such as `set`, since the import would shadow the builtin
  ([#94](https://github.com/gaborbernat/simplint/pull/94)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_context_var_token_reset.md)
  and [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md) keep
  their report without a fix when another module reads the module-level name they would remove, through an import, a
  star import or an attribute, or when `__all__` lists it, so the fix no longer breaks that module
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-variable-prefix`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/module_variable_prefix.md)
  no longer asks to rename `_` to an empty name or `_class` to the keyword `class`. The fix for those names used to be
  rejected as a syntax error, and a debug build panicked ([#110](https://github.com/gaborbernat/simplint/pull/110)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--fix` and `--diff` warn about a `noqa` comment Ruff cannot read once, with the line it has in the file on disk,
  instead of once for each intermediate version of the file, each with line numbers from text that is not on disk
  ([#100](https://github.com/gaborbernat/simplint/pull/100)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-returns`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_returns.md) (SMP923) no
  longer reports the `:yields:` of an abstract generator whose body is a placeholder, and no longer asks for `:returns:`
  there. Its fix deleted the `:yields:` field ([#116](https://github.com/gaborbernat/simplint/pull/116)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A cached clean result of
  [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/private_import_path.md)
  no longer stands once the source module a test mirrors is deleted: a warm run reports the test's private import as a
  run without the cache does ([#100](https://github.com/gaborbernat/simplint/pull/100)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A test's private import is checked again when the source file it mirrors is deleted from a directory outside Ruff's
  `src` roots, so a stale clean result no longer hides
  [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/private_import_path.md)
  (SMP310) ([#108](https://github.com/gaborbernat/simplint/pull/108)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-re-error-alias`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_re_error_alias.md) and
  [`up-re-prefixmatch`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_re_prefixmatch.md) leave
  a `re` alone when another import of that name, such as `import regex as re` in a `try` block, binds a different
  module, so the fix no longer renames an `except` clause to a name the other module lacks
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unnamed-regex-group`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unnamed_regex_group.md) no
  longer makes the groups of a pattern non-capturing when the code reads them by number through a class attribute, a
  name declared `global` or `nonlocal`, or a module name that another module imports or `__all__` lists, reads the rule
  does not follow ([#99](https://github.com/gaborbernat/simplint/pull/99)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unnamed-regex-group`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unnamed_regex_group.md)
  (SMP532) and
  [`unexplained-regex`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unexplained_regex.md) now
  read a `#` comment in a scoped `(?x:...)` group as a comment, so a parenthesis in it is no longer counted as a group.
  A `(?-x:...)` group under `re.VERBOSE` is read as not verbose, so its `#` and parentheses count
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  and [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/class_member_order.md)
  keep the blank lines between two statements that stay adjacent in the same order. The fix used to replace them with
  the usual separator ([#110](https://github.com/gaborbernat/simplint/pull/110)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-unknown-attribute`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_unknown_attribute.md)
  no longer reports the `:ivar:` and `:cvar:` fields of names a class body unpacks from a tuple, or of the strings its
  `__slots__` declares, as attributes the class lacks ([#99](https://github.com/gaborbernat/simplint/pull/99)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-unknown-param`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_unknown_param.md)
  deletes a field from a docstring whose remaining text ends in a quote or a backslash, and puts the closing quotes on a
  line of their own. The fix used to join them to that text, which ends or escapes the string, so the fix was rejected
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-missing-param`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_missing_param.md)
  and
  [`rst-attribute-kind`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_attribute_kind.md)
  read a quoted annotation such as `"ClassVar[int]"` as the `ClassVar` it spells, so a class variable declared that way
  needs no `:param:` field, and an `:ivar:` field for it is reported
  ([#99](https://github.com/gaborbernat/simplint/pull/99)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-returns`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_returns.md) no longer
  treats a function as a generator when a lambda in its body yields, so its `:returns:` field is checked as a return
  field ([#99](https://github.com/gaborbernat/simplint/pull/99)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-object-sentinel`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_object_sentinel.md)
  judges a class attribute by the other bindings of its class body, so `Config.MISSING` assigned twice in `Config` is no
  longer reported, while `Other.LIMIT = object()` is reported even when the module binds `LIMIT` twice
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-object-sentinel`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_object_sentinel.md)
  names a class attribute with its class, as `sentinel("Config.UNSET")`, so pickling can find it, and no longer offers a
  fix for a placeholder in a class inside a function ([#99](https://github.com/gaborbernat/simplint/pull/99)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md),
  [`up-asyncio-iscoroutinefunction`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_asyncio_iscoroutinefunction.md),
  [`up-codecs-open`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_codecs_open.md) and
  [`up-removed-ast-node`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_removed_ast_node.md)
  keep their report without a fix when a parameter or local, a loop variable or a lambda parameter of a scope that
  encloses the call takes the module or builtin name the rewrite uses, so the rewritten code no longer calls that
  binding. A nested function or a comprehension elsewhere in the function does not count
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md),
  [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/module_attribute_import.md)
  and
  [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  keep their autofix available when the first report is suppressed and a later report is still visible
  ([#106](https://github.com/gaborbernat/simplint/pull/106)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_use_variable.md)
  keeps the order of evaluation when it inlines a value: a fix that moves a call, a lookup or a walrus past the read is
  now unsafe, and a read in a t-string is reported without a fix. A string with braces that replaces a field in a format
  spec, as in `{v:{inner}}`, no longer changes what the f-string prints
  ([#102](https://github.com/gaborbernat/simplint/pull/102)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_use_variable.md)
  (SMP504) no longer inlines a string into a field of a nested f-string before Python 3.12 when the string holds the
  quote of an enclosing f-string. The fix had made the file fail to parse, which dropped every other fix in it
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_use_variable.md)
  (SMP504) no longer inlines a value into a `=` debug field of a raw f-string whose debug text holds the string's quote.
  The fix wrote the quote with a backslash, which a raw f-string keeps, so the printed text changed
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/single_user_constant.md)
  no longer indents the next line of a string that a backslash continues, since the spaces became part of the string's
  value. The constant is still reported, without a fix ([#110](https://github.com/gaborbernat/simplint/pull/110)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/single_user_constant.md)
  inlines an integer constant as `(30).bit_length()` when an attribute follows it. The fix used to write
  `30.bit_length()`, which does not parse, so the fix was rejected
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unused_default.md) counts a
  positional call that suppresses
  [`dataclass-kw-only`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/dataclass_kw_only.md) like
  any other call, instead of treating it as a use it cannot follow and keeping the function's defaults
  ([#89](https://github.com/gaborbernat/simplint/pull/89)) by [@gaborbernat](https://github.com/gaborbernat).
- [`function-docstring`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/function_docstring.md)
  drops `Params:` and `Yield:` sections when it turns a docstring into a comment, as the reST docstring rules already
  read them, and keeps a line such as `: name: text`, which reST does not parse as a field
  ([#89](https://github.com/gaborbernat/simplint/pull/89)) by [@gaborbernat](https://github.com/gaborbernat).
- [`rst-field-syntax`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_field_syntax.md)
  (SMP910) and
  [`rst-missing-param`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_missing_param.md)
  (SMP916) read `:kwarg:` and `:kwparam:` as spellings of `:param:`, and `:paramtype:` as one of `:type:`, as Sphinx
  does, instead of reporting them as unknown fields ([#116](https://github.com/gaborbernat/simplint/pull/116)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-type-field`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_type_field.md)
  (SMP912) keeps the stars of `*args` and `**kwargs` when its fix drops the type from `:param int \*args:`, which before
  wrote `:param args:` ([#116](https://github.com/gaborbernat/simplint/pull/116)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A line of a multi-line string that starts with `#` is no longer taken for a comment above the next statement. A move
  of
  [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  or [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/class_member_order.md)
  used to pull that line along and split the string, which changed its value
  ([#110](https://github.com/gaborbernat/simplint/pull/110)) by [@gaborbernat](https://github.com/gaborbernat).
- `--stdin-filename` applies the file filters a file on disk gets: a notebook, TOML or Markdown name, or a name
  `--force-exclude` leaves out, reports nothing and exits 0. With `--fix`, its source goes out unchanged. A `.pyi` name
  is still linted, though a stub on disk is only indexed ([#108](https://github.com/gaborbernat/simplint/pull/108)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `check` reads standard input as Ruff does: with `--stdin-filename` the source is standard input whatever paths come
  with it, and a `-` beside other paths is no longer standard input. `--add-noqa` and `--add-ignore` refuse standard
  input instead of editing the files under the working directory
  ([#100](https://github.com/gaborbernat/simplint/pull/100)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-strptime-date`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_strptime_date.md) no
  longer applies a safe fix to `.time()` on a format with `%:z`, since `time.strptime` keeps the zone that `.time()`
  drops, as it does with `%z` ([#99](https://github.com/gaborbernat/simplint/pull/99)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-strptime-date`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_strptime_date.md) makes
  the `.time()` fix unsafe when the format has a date directive, and the `.date()` and `.time()` fixes unsafe when it
  has a seconds directive. A safe `--fix` used to rewrite `datetime.strptime(text, "%m-%d %H:%M").time()` into a call
  that accepts February 29 in a year without a leap day, which the original call rejects with `ValueError`, and
  `datetime.strptime(text, "%H:%M:%S").time()` into a call that accepts second 60
  ([#115](https://github.com/gaborbernat/simplint/pull/115)) by [@gaborbernat](https://github.com/gaborbernat).
- [`super-forwarding-method`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/super_forwarding_method.md)
  (SMP527) no longer reports a method whose name the class body reads or binds again, such as `__str__ = describe` or an
  import under an `if`. Deleting such a method made the class body look the name up in the module, so the fix broke the
  import with a `NameError` ([#111](https://github.com/gaborbernat/simplint/pull/111)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`super-forwarding-method`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/super_forwarding_method.md)
  no longer deletes a comment inside the method it would remove, such as one on its `return` line. A method with such a
  comment is left alone, as one above it already was ([#93](https://github.com/gaborbernat/simplint/pull/93)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`test-class`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/test_class.md) dedents each
  method by the indentation of its class body rather than the file's first indentation. A two-space function above a
  four-space `class TestThing` used to keep the method nested in that function, where pytest does not collect it
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-class`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/test_class.md) (SMP801) no longer
  flattens onto a function name that another class or another method of the same class already takes, which shadowed a
  test. The first class keeps its fix; a later class, or a class with two methods of one name, is reported without one
  ([#109](https://github.com/gaborbernat/simplint/pull/109)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-class`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/test_class.md) reports without a
  fix when a method of `class Test` would take the name of a module-level function. The fix used to add a second
  definition under that name, which replaced the module-level one
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-only-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/test_only_code.md) no longer
  reports the functions of a module that production code passes on as a value, as `codec = fast` or a default argument
  does, since the code calls them through that value ([#101](https://github.com/gaborbernat/simplint/pull/101)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`test-only-parameter`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/test_only_parameter.md)
  (SMP410) counts the calls of a module that production code star-imports, or that has an `if __name__ == "__main__":`
  block, as production calls, even when a test imports it. It no longer reports a parameter that such a module passes
  ([#118](https://github.com/gaborbernat/simplint/pull/118)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-only-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/test_only_code.md) (SMP408) also
  keeps the functions of a module that production code reaches as `app.fast`, that a class body imports, or that a
  function body imports and loads as a value itself, since the code calls them through that module. A function that only
  calls the module's functions does not keep the others ([#118](https://github.com/gaborbernat/simplint/pull/118)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md) (SMP401)
  no longer reports a name that only tests import when production code loads its module as a value, as
  `{"compat": compat}` does. The fix would have listed the name in `__all__`
  ([#118](https://github.com/gaborbernat/simplint/pull/118)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-traceback-exception-arguments`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_traceback_exception_arguments.md)
  removes the parentheses around the replaced arguments too. A parenthesized argument used to leave one behind, and the
  fix was then reverted for the whole file ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-warnings-deprecated`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_warnings_deprecated.md)
  keeps a comment that trails the `warnings.warn` call: with `--unsafe-fixes` the report now stays without a fix instead
  of deleting the comment with the line ([#115](https://github.com/gaborbernat/simplint/pull/115)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-type-checking-lazy-import`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/up_type_checking_lazy_import.md)
  no longer adds a second `lazy` to an import that is `lazy` already, which produced a syntax error and reverted the
  fixes of the file ([#114](https://github.com/gaborbernat/simplint/pull/114)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A file that cannot be read, while `io-error` (E902) is not selected, is no longer cached as clean, so every run logs
  its warning again ([#108](https://github.com/gaborbernat/simplint/pull/108)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-default`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unused_default.md) (SMP506)
  still reports a default that every call passes, but no longer offers a fix that deletes a comment inside that default
  ([#111](https://github.com/gaborbernat/simplint/pull/111)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) no longer
  reports the names of an `__all__` that another module imports or reads, as `from module import __all__` or
  `module.__all__` does. Such a module uses every name the list holds, so `--fix` no longer removes them
  ([#101](https://github.com/gaborbernat/simplint/pull/101)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) (SMP402) no
  longer turns off for the whole run when a file reads `__all__` off a module it cannot name, such as
  `sys.modules["json"]`. Only an import of `__all__` or a read of it off a module of the run exempts a module's
  `__all__` ([#118](https://github.com/gaborbernat/simplint/pull/118)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) removes an
  unused name in parentheses from `__all__`, as in `__all__ = "zeta", ("beta")`, without leaving a parenthesis behind.
  The fix used to leave one, which does not parse, so the fix was rejected
  ([#98](https://github.com/gaborbernat/simplint/pull/98)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) (SMP402) no
  longer reports a name of a module that production code loads as a value, as `CONFIG = settings` does, or a submodule
  that a star import binds through its package's `__all__`. A test that passes the module on exempts nothing
  ([#118](https://github.com/gaborbernat/simplint/pull/118)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) (SMP402) reports
  without a fix when a removed name sits alone on its line with a comment after it. The fix used to move that comment
  onto the name before it ([#113](https://github.com/gaborbernat/simplint/pull/113)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/comments/unused_ignore.md) (SMP009)
  removes only the directive and its reason when the last selector of a comment is unused. A comment after them on the
  line, such as `# noqa: E501`, stays instead of being deleted with them
  ([#93](https://github.com/gaborbernat/simplint/pull/93)) by [@gaborbernat](https://github.com/gaborbernat).
- [`walrus-candidate`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/walrus_candidate.md) no
  longer inlines a name that `locals()`, `vars()`, `eval` or `exec` reads in its function, or that `globals()` reads at
  module level, since the call would no longer find it ([#102](https://github.com/gaborbernat/simplint/pull/102)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A file `--fix` cannot write, such as a read-only one, keeps its reports: the run shows them with the write error, and
  a later run no longer caches the file as clean and exits 0 ([#100](https://github.com/gaborbernat/simplint/pull/100))
  by [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-2-performance"></a>

### Performance

- [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md) (SMP401)
  and [`unused-export`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_export.md) (SMP402)
  attach their fix to the first report only, instead of copying the whole listing into every report. A module with many
  names no longer grows its output and memory quadratically ([#93](https://github.com/gaborbernat/simplint/pull/93)) by
  [@gaborbernat](https://github.com/gaborbernat).
- The rules that ask whether `__all__` lists a name check a set of the listed names, built once per file, instead of
  scanning the list for each name:
  [`missing-from-all`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/missing_from_all.md) (SMP401),
  [`unexported-public-name`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unexported_public_name.md)
  (SMP403), [`test-only-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/test_only_code.md)
  (SMP408),
  [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_definition.md)
  (SMP409),
  [`test-only-parameter`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/test_only_parameter.md)
  (SMP410) and
  [`unused-return-value`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_return_value.md)
  (SMP411). A module of 64,000 functions, all listed in `__all__`, takes 0.09 seconds under SMP401 instead of 2.8
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-asyncio-iscoroutinefunction`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_asyncio_iscoroutinefunction.md)
  looks up the imports of the module in a set built once per file, instead of scanning the module body for each call. A
  module of 32,000 `asyncio.iscoroutinefunction()` calls takes 0.10 seconds instead of 1.5
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  and [`class-member-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/class_member_order.md)
  compare each class statement with the others by name instead of with every statement, and share one build of each
  class order, so a class of 64,000 attributes takes 0.05 seconds under SMP201 instead of 2.8 and 0.06 under SMP203
  instead of 2.6 ([#104](https://github.com/gaborbernat/simplint/pull/104)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-compression-modules`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_compression_modules.md)
  reads the attributes of the file once and checks each codec and name once, instead of walking the whole module for
  each import. A module of 32,000 functions that each import `zlib` takes 0.11 seconds instead of 18.0
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- Collecting the fields of a `@dataclass` checks each new field against a set instead of the fields before it, so a
  dataclass with 64,000 fields takes 0.06 seconds instead of 3.9
  ([#96](https://github.com/gaborbernat/simplint/pull/96)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_use_variable.md)
  finds the `=` debug fields that an edit lies in with a search over the fields of the file, and builds its groups of
  edits only for the fields that get one. A file of 20,000 inlinable pairs and 20,000 `f"{s=}"` fields takes 0.10
  seconds under SMP504 instead of 1.7 ([#123](https://github.com/gaborbernat/simplint/pull/123)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  finds the annotations that a decorator reads with a binary search over the sorted annotation ranges instead of a scan
  of all of them, so 32,000 decorated functions take 0.12 seconds instead of 4.2
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- [`definition-before-use`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/layout/definition_before_use.md)
  follows each chain of helpers that a function is used from once, instead of once from every function in it. A chain of
  64,000 helpers takes 0.20 seconds under SMP201 instead of 2.6
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- Checking whether the documentation names a definition in a file outside any package, as in a `tests/` or `scripts/`
  directory without `__init__.py`, no longer builds a string for each documented name: 10,000 such definitions against
  20,000 documented names take 0.11 seconds instead of 0.6 ([#88](https://github.com/gaborbernat/simplint/pull/88)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`duplicate-code`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/duplicate_code.md) pairs a
  repeated block with three of the modules that repeat it instead of with every one of them, and still counts all of
  them in its report: 4,000 modules that each repeat the same eight statements take 0.1 seconds instead of 30.4
  ([#96](https://github.com/gaborbernat/simplint/pull/96)) by [@gaborbernat](https://github.com/gaborbernat).
- [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/early_assignment.md) searches
  the scopes nested in a function by offset instead of testing each mention against all of them. A function of 80,000
  lambdas takes 0.10 seconds under SMP526 instead of 2.3 ([#123](https://github.com/gaborbernat/simplint/pull/123)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/early_assignment.md) searches
  the string tokens and the statements of a block by position instead of scanning them for each constant, so 32,000
  constants assigned above their uses take 0.05 seconds instead of 1.8
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-unittest-enter-context`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/up_unittest_enter_context.md)
  counts the names of each test method once instead of scanning the tokens of the whole file for every patch it checks,
  so 16,000 manual patch sequences take 0.15 seconds instead of 3.3
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- Finding the exception classes a file's names can refer to, which
  [`indistinct-error-class`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/classes/indistinct_error_class.md)
  relies on, no longer checks every exception class of the run in every file: 16,000 modules that each define an
  exception class take 0.3 seconds instead of 10.1 ([#96](https://github.com/gaborbernat/simplint/pull/96)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Finding the exception classes of the project follows each class to its subclasses once, instead of passing over every
  class for each level of inheritance. A chain of 16,000 modules, each subclassing the exception of the one before,
  takes 0.96 seconds instead of 5.2 ([#112](https://github.com/gaborbernat/simplint/pull/112)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Fixing a file with many withdrawn fixes no longer holds every intermediate source in memory: 1,600 withdrawn fixes of
  one rule peak at 212 MiB instead of 4.3 GiB ([#95](https://github.com/gaborbernat/simplint/pull/95)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--fix` runs that rename names across a large project use less memory: the words of each file kept between fix rounds
  now take about 8 bytes each, which cut peak memory from 830 MB to 474 MB on 3,000 word-heavy modules
  ([#88](https://github.com/gaborbernat/simplint/pull/88)) by [@gaborbernat](https://github.com/gaborbernat).
- Files with many `# fmt: skip` comments are checked faster: a 20,000-line module with one on every line takes a quarter
  of the time it did under `--fix` ([#88](https://github.com/gaborbernat/simplint/pull/88)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md) keeps the
  modules of a cycle as ids and names only the first eight of a long one, counting the rest, so a ring of 4,000 modules
  that import each other in turn takes 0.7 seconds and 172 MB instead of 2.4 seconds and 3.3 GB
  ([#96](https://github.com/gaborbernat/simplint/pull/96)) by [@gaborbernat](https://github.com/gaborbernat).
- [`import-cycle`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/imports/import_cycle.md) finds the
  cycle of a ring of modules without a search from each module and keeps only the shown files of each long cycle in
  memory: a ring of 32,000 modules takes 2.1 seconds and 0.5 GB instead of 50.9 seconds and 4.8 GB
  ([#112](https://github.com/gaborbernat/simplint/pull/112)) by [@gaborbernat](https://github.com/gaborbernat).
- [`loop-in-test`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/loop_in_test.md) follows each
  chain of local names that a test's table is built from once instead of once per path through it, so eight chained
  names with twelve references each take under 0.01 seconds instead of 16.4
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- [`mock-assertion-typo`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/testing/mock_assertion_typo.md)
  checks each misspelled name against a set of the file's function names instead of scanning every function for each
  one, so 32,000 misspelled calls take 0.14 seconds instead of 0.9
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- A file that imports a module of the run with `import module` shares that module's members with the index instead of
  copying them for each importer: 8,000 modules that import a module with 8,000 functions take 0.16 seconds and 128 MB
  instead of 2.2 seconds and 4.9 GB ([#96](https://github.com/gaborbernat/simplint/pull/96)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A cached run lists the directories under each project and search root, to tell whether the documentation or the import
  layout changed, on all threads instead of one, so a project with a large `node_modules/` or gitignored `target/` tree
  no longer spends most of a cached run walking it: with 20,000 such directories a fully cached run takes 0.14 seconds
  instead of 0.22 ([#88](https://github.com/gaborbernat/simplint/pull/88)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Looking up what an imported name refers to no longer scans all of the importing file's imports for each name, so a
  module of 64,000 top-level `from` imports takes 0.56 seconds instead of 13.9
  ([#112](https://github.com/gaborbernat/simplint/pull/112)) by [@gaborbernat](https://github.com/gaborbernat).
- Following a name through a chain of modules that re-export it remembers where each module leads, so every module of
  the chain no longer walks the rest of it: 8,000 packages that each re-export the name of the one before take 0.2
  seconds instead of 4.6 ([#96](https://github.com/gaborbernat/simplint/pull/96)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Looking up a re-exported class is memoised again in the files that reach no import cycle, even when the project has
  one elsewhere. A chain of 16,000 re-exporting packages with two unrelated modules that import each other takes 1.4
  seconds instead of 20.3 ([#112](https://github.com/gaborbernat/simplint/pull/112)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`rst-param-order`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_param_order.md) and
  [`rst-type-field`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_type_field.md) look up
  the parameter of each field in a map instead of scanning the signature, and
  [`rst-unknown-attribute`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/docstrings/rst_unknown_attribute.md)
  finds each attribute's earlier definition by name, so 32,000 parameters or attributes take 0.04 and 0.06 seconds
  instead of 0.46 and 0.57 ([#104](https://github.com/gaborbernat/simplint/pull/104)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_chdir_restore.md) and
  [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_context_var_token_reset.md)
  find the function or class around a statement with a search over the file's scopes instead of a scan of all of them. A
  module of 32,000 functions with a `finally` block each takes 0.22 seconds under SMP512 instead of 1.2, and 5.2 seconds
  under SMP513 instead of 7.6 ([#123](https://github.com/gaborbernat/simplint/pull/123)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`single-caller-helper`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/single_caller_helper.md)
  finds the caller of each helper in a map of the file's functions instead of scanning them, and checks the size of a
  caller before walking it for loops, so 64,000 helpers take 0.31 seconds instead of 2.9 and a caller with 32,000 calls
  to helpers takes 0.14 seconds instead of 8.8 ([#104](https://github.com/gaborbernat/simplint/pull/104)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`walrus-candidate`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/walrus_candidate.md) looks up
  the other mentions of each name in one index of the module instead of walking every statement before it, so its time
  no longer grows with the square of a module's assignments: 16,000 assignment and `if` pairs take 0.06 seconds instead
  of 9.9 ([#88](https://github.com/gaborbernat/simplint/pull/88)) by [@gaborbernat](https://github.com/gaborbernat).
- A module that star-imports other modules no longer walks the whole chain of star imports behind it again for each
  importer when looking for exception classes: 16,000 modules, each star-importing the one before, take 0.76 seconds
  instead of 3.8 ([#112](https://github.com/gaborbernat/simplint/pull/112)) by
  [@gaborbernat](https://github.com/gaborbernat).
- Following names through chains of `import *` re-exports looks each name up in a set of the source's `__all__` and
  revisits only the modules that gained names, so a chain of 200 modules whose `__all__` lists the whole chain takes
  0.14 seconds instead of 6.8 ([#88](https://github.com/gaborbernat/simplint/pull/88)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`string-concatenation`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/string_concatenation.md)
  reads the bindings of a scope in one walk for all of its names, instead of one walk per name. A module of 16,000
  annotated classes, each used by a function, takes 0.16 seconds under SMP503 instead of 2.9
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-strptime-date`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/up_strptime_date.md) counts
  the identifiers of the file once instead of scanning every token of it for each `strptime` call. A module of 32,000
  `datetime.strptime(...).date()` calls takes 0.12 seconds instead of 6.7
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`super-forwarding-method`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/super_forwarding_method.md)
  looks up the bindings and the classes of a module in indexes built once per file, instead of scanning the class bodies
  for each method. A class of 64,000 forwarding methods takes 0.32 seconds instead of 35.5
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`suppression-without-reason`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/comments/suppression_without_reason.md)
  looks up the reason of each suppression in one map of the file's comment lines instead of scanning every comment run
  for it, so 64,000 suppressions take 0.05 seconds instead of 4.8
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unnamed-regex-group`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/unnamed_regex_group.md)
  decides once per name whether the code reads a group by index, instead of scanning that name's uses for each pattern
  bound to it. The repro at 80,000 patterns takes 1.6 seconds under SMP532 instead of 3.1
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- A file that imports a module outside the run, such as one from the standard library, no longer compares its module
  name with the path of every file of the run: 16,000 files that import and call standard-library modules take 0.4
  seconds instead of 1.6 ([#96](https://github.com/gaborbernat/simplint/pull/96)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_definition.md) finds
  the names a class body reads in one walk, instead of walking the whole body for each method and attribute. A class of
  32,000 unused methods takes 0.08 seconds under SMP409 instead of 10.2
  ([#123](https://github.com/gaborbernat/simplint/pull/123)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-definition`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/api/unused_definition.md) reads
  the words of a file's doctests once instead of rescanning every docstring for each definition it checks, so 16,000
  documented functions take 0.06 seconds instead of 3.1 ([#104](https://github.com/gaborbernat/simplint/pull/104)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`unused-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/comments/unused_ignore.md) (SMP009)
  checks the selectors of a file together and lints it again only for the sets whose removal brings a report back,
  instead of once per selector. On a 900-line file with an ignore comment on every line the run drops from 2.4 s to 22
  ms ([#88](https://github.com/gaborbernat/simplint/pull/88)) by [@gaborbernat](https://github.com/gaborbernat).
- [`walrus-candidate`](https://github.com/gaborbernat/simplint/blob/0.10.2/src/rules/logic/walrus_candidate.md) reads
  the names a function declares `global` or `nonlocal` once per function instead of once per assignment it checks, so
  32,000 assignment and `if` pairs in one function take 0.45 seconds instead of 4.7
  ([#104](https://github.com/gaborbernat/simplint/pull/104)) by [@gaborbernat](https://github.com/gaborbernat).
- A cached run no longer rebuilds the project index when one file has a syntax error or is not valid UTF-8: a warm run
  over pandas with such a file takes 0.20 seconds instead of 0.79
  ([#95](https://github.com/gaborbernat/simplint/pull/95)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.10.1 (2026-11-10)

<a name="release-0-10-1"></a>

<a name="release-0-10-1-fixed"></a>

### Fixed

- `--add-ignore` no longer adds `numeric-ignore` to a comment whose numeric codes it renames, which left a selector that
  [`unused-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/comments/unused_ignore.md) (SMP009)
  then reported ([#83](https://github.com/gaborbernat/simplint/pull/83)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `--add-noqa` and `--add-ignore` keep what follows the codes of a comment they merge into: its reason and any later
  comment, such as `# type: ignore[import]` or `# pragma: no cover`. A reason passed to the flag replaces only the old
  reason ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- A `.ruff_cache` directory, or the `simplint` directory in it, that a link in the repository points outside the project
  no longer receives simplint's cache, its `CACHEDIR.TAG` or its `.gitignore`; the run warns and does not cache
  ([#85](https://github.com/gaborbernat/simplint/pull/85)) by [@gaborbernat](https://github.com/gaborbernat).
- The cache keys each file by its size as well as its modification time and permissions, so a file rewritten within one
  tick of a coarse clock, or copied with its modification time kept (`cp -p`, `rsync -t`), is checked again instead of
  reported from its old cached result ([#85](https://github.com/gaborbernat/simplint/pull/85)) by
  [@gaborbernat](https://github.com/gaborbernat).
- A cache file that was damaged or edited by hand, holding a range that ends before it starts or past the end of its
  file, is treated as a cache miss instead of crashing the run ([#85](https://github.com/gaborbernat/simplint/pull/85))
  by [@gaborbernat](https://github.com/gaborbernat).
- Cached import facts are collected again when a file's module path changes, as when Ruff's `src` setting moves the
  search roots, instead of resolving relative imports against the old path until the file changes
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- Fixes that insert lines, such as moving definitions, adding `__all__` or imports, flattening test classes and moving
  constants into functions, write CRLF line endings in files that use them instead of bare LF
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- An `extend`ed configuration that a link in the repository points outside the project is now named without its text
  when Ruff cannot load it, as one outside the project by path already was
  ([#85](https://github.com/gaborbernat/simplint/pull/85)) by [@gaborbernat](https://github.com/gaborbernat).
- A `# ruff: file-ignore[...]` comment suppresses simplint rules by name, such as `single-letter-name`, as a line's
  `# ruff: ignore[...]` already did; before, only codes matched ([#83](https://github.com/gaborbernat/simplint/pull/83))
  by [@gaborbernat](https://github.com/gaborbernat).
- When a fix breaks syntax or does not converge only in combination with other withdrawn fixes, simplint drops that
  combination and keeps fixing the file, instead of reverting every fix in it and printing the bug report once per
  attempt ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- `--fix` writes the fixed source into the file in place, as Ruff does, so the file keeps its owner, hard links, ACLs
  and extended attributes instead of being replaced by a new file owned by whoever ran simplint
  ([#85](https://github.com/gaborbernat/simplint/pull/85)) by [@gaborbernat](https://github.com/gaborbernat).
- [`absolute-sibling-import`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/imports/absolute_sibling_import.md)
  (SMP302) looks for a flake8-lazy hook only up to the linted project when no repository holds it, so a
  `.pre-commit-config.yaml` or `prek.toml` in the home directory no longer turns the rule off. A hook configuration or a
  `hatch.toml` that cannot be read or parsed counts as absent, with a warning, instead of stopping the run
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- A file the run finds both by its own path and through a link inside the project is linted and fixed once, by its own
  path, instead of twice in parallel, where the second write failed and the reports showed twice
  ([#85](https://github.com/gaborbernat/simplint/pull/85)) by [@gaborbernat](https://github.com/gaborbernat).
- The fixes of
  [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/up_context_var_token_reset.md),
  [`up-chdir-restore`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/up_chdir_restore.md),
  [`up-warnings-deprecated`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/up_warnings_deprecated.md)
  and [`early-assignment`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/early_assignment.md)
  write the file's own line ending, so a CRLF file no longer gets LF lines mixed in
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/layout/single_user_constant.md)
  keeps a list, set or dict at module level when the function returns it, passes it to another function or binds another
  name to it, since a fresh copy on each call would lose the state other code changed
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- `private-import-path` accepts imports from a matching source file when the test mirrors a private package path or
  lives in a test subtree named after that module. The test tree may omit the top-level package directory; a `.pyi` can
  identify a compiled module ([#81](https://github.com/gaborbernat/simplint/pull/81)) by
  [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` leaves names used only by tests out of `__all__`, so testing a private implementation does not make
  it public ([#81](https://github.com/gaborbernat/simplint/pull/81)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/single_use_variable.md)
  marks its fix unsafe when the value calls something and the next statement first formats or unpacks a name, or reads a
  name declared `global` or `nonlocal`, since the call could change what that read sees
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-context-var-token-reset`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/up_context_var_token_reset.md)
  only reports a `set`/`reset` pair on a name assigned a `contextvars.ContextVar` or a parameter annotated as one, so
  another class's `set` is no longer rewritten into a `with` block that raises `TypeError`
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- [`up-re-error-alias`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/logic/up_re_error_alias.md) leaves
  `re.error = ...` and `del re.error` alone, which patch the module rather than read the exception
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- [`test-class`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/testing/test_class.md) keeps the comment
  lines directly above each method when it flattens a class, and offers no fix when a comment in the class belongs to no
  method. The fix used to drop them ([#83](https://github.com/gaborbernat/simplint/pull/83)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`mock-assertion-typo`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/testing/mock_assertion_typo.md)
  only fixes a misspelled assertion called on a line of its own or as the whole test of an `assert`, and drops that
  `assert`. Elsewhere the rename left a bound method that is always true, or a `None` the code went on to use
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- `module-variable-prefix` keeps the declared name of a matching `TypeVar`, `TypeVarTuple` or `ParamSpec` assignment
  ([#84](https://github.com/gaborbernat/simplint/pull/84)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-letter-name` accepts conventional type parameter names in PEP 695 declarations and matching `TypeVar`,
  `TypeVarTuple` and `ParamSpec` assignments ([#84](https://github.com/gaborbernat/simplint/pull/84)) by
  [@gaborbernat](https://github.com/gaborbernat).
- [`module-variable-case`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/naming/module_variable_case.md)
  and
  [`single-user-constant`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/layout/single_user_constant.md)
  change the case of non-ASCII letters too, so `café_rate` becomes `CAFÉ_RATE` instead of `CAFé_RATE`
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unused-ignore`](https://github.com/gaborbernat/simplint/blob/0.10.1/src/rules/comments/unused_ignore.md) (SMP009)
  reports unused numeric selectors while `numeric-ignore` (SMP008) is on. SMP008 reports on the codes each removal
  edits, which kept every numeric selector until SMP008's fix renamed it
  ([#83](https://github.com/gaborbernat/simplint/pull/83)) by [@gaborbernat](https://github.com/gaborbernat).

## 0.10.0 (2026-11-10)

<a name="release-0-10-0"></a>

<a name="release-0-10-0-added"></a>

### Added

- `unused-ignore` reports unused `simplint: ignore` selectors and removes them with a safe fix
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- [`entrypoint-module-name`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/naming/entrypoint_module_name.md)
  reports command entrypoints in `main.py` or `_main.py` so they can move to `__main__.py`
  ([#76](https://github.com/gaborbernat/simplint/pull/76)) by [@gaborbernat](https://github.com/gaborbernat).
- [`unguarded-entrypoint-call`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/naming/unguarded_entrypoint_call.md)
  uses script metadata, shebangs, executable bits, and `__main__.py` to report trailing entrypoint calls regardless of
  their name. Single-line statements have an unsafe guard fix ([#78](https://github.com/gaborbernat/simplint/pull/78))
  by [@gaborbernat](https://github.com/gaborbernat).
- [`private-import-path`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/imports/private_import_path.md)
  reports private module-object imports and private imports outside a direct relative path, including nested names such
  as `._x._y`. Tests may import names from a private module only when their package path and `test_` stem match the
  source module ([#74](https://github.com/gaborbernat/simplint/pull/74)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-0-changes"></a>

### Changes

- Removed `up-unittest-assert-helper`: Ruff's
  [`pytest-unittest-assertion`](https://docs.astral.sh/ruff/rules/pytest-unittest-assertion/) reports every assertion it
  covered when Ruff rules are all enabled ([#75](https://github.com/gaborbernat/simplint/pull/75)) by
  [@gaborbernat](https://github.com/gaborbernat).

<a name="release-0-10-0-fixed"></a>

### Fixed

- [`constant-assert`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/testing/constant_assert.md) leaves
  tuple and string literal assertions to Ruff's F631 and PLW0129 even when simplint could fix them
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-variable-case`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/naming/module_variable_case.md)
  leaves `mixedCase` names to Ruff's N816 even when simplint could rename them
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- [`single-use-variable`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/logic/single_use_variable.md)
  leaves `yield from` assignments to Ruff's RUF070 even when Ruff's fix differs
  ([#77](https://github.com/gaborbernat/simplint/pull/77)) by [@gaborbernat](https://github.com/gaborbernat).
- `single-user-constant` respects Ruff's enabled local-variable limit before suggesting moving a constant into a
  function ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- [`module-attribute-import`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/imports/module_attribute_import.md)
  reports private module attribute access even when Ruff selects `PLC2701`, which checks imports rather than attribute
  access ([#74](https://github.com/gaborbernat/simplint/pull/74)) by [@gaborbernat](https://github.com/gaborbernat).
- `missing-from-all` leaves `__main__.py` out of the missing-export check
  ([#72](https://github.com/gaborbernat/simplint/pull/72)) by [@gaborbernat](https://github.com/gaborbernat).
- [`init-module-content`](https://github.com/gaborbernat/simplint/blob/0.10.0/src/rules/api/init_module_content.md)
  accepts package re-exports listed in `__all__`, including in applications and private subpackages
  ([#73](https://github.com/gaborbernat/simplint/pull/73)) by [@gaborbernat](https://github.com/gaborbernat).
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
