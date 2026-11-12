use ruff_db::diagnostic::Diagnostic;
use ruff_linter::FixAvailability;
use ruff_macros::CacheKey;
use ruff_python_ast::{self as ast, Expr, Pattern, PythonVersion, Stmt, TypeParam};

use std::sync::LazyLock;

use crate::lint::checker::Checker;
use crate::lint::walker::{Scope, Walker};
use crate::project::Parts;
use crate::rules;

// Each rule lists its hooks: `module` runs the rule's function once over the whole module; every other hook names the
// kind of node the walker hands to the function of that name in the rule's module.
macro_rules! rules {
    ($($variant:ident => (
        $code:literal, $name:literal, $summary:literal, $fix:ident, $family:ident::$check:ident { $($hook:ident),* }
        $(since $min:ident)?
        $(reads [$($read:ident), *])?
        $(skips [$($skip:ident), *])?
    ),)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Rule {
            $($variant,)*
        }

        impl Rule {
            pub const ALL: &[Rule] = &[$(Rule::$variant,)*];

            pub fn code(self) -> &'static str {
                match self {
                    $(Rule::$variant => $code,)*
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(Rule::$variant => $name,)*
                }
            }

            pub fn summary(self) -> &'static str {
                match self {
                    $(Rule::$variant => $summary,)*
                }
            }

            pub fn explanation(self) -> &'static str {
                match self {
                    $(Rule::$variant => include_str!(concat!(
                        "../rules/", stringify!($family), "/", stringify!($check), ".md"
                    )),)*
                }
            }

            pub fn from_code(code: &str) -> Option<Self> {
                Self::from_code_or_name(code).filter(|rule| rule.code() == code)
            }

            pub fn from_code_or_name(value: &str) -> Option<Self> {
                match value {
                    $($code | $name => Some(Rule::$variant),)*
                    _ => None,
                }
            }

            pub fn url(self) -> String {
                let url = match self {
                    $(Rule::$variant => concat!("https://simplint.readthedocs.io/en/latest/rules/", $code, ".html"),)*
                };
                url.to_owned()
            }

                    pub fn family(self) -> &'static str {
                match self {
                    $(Rule::$variant => stringify!($family),)*
                }
            }

            pub fn fix_availability(self) -> FixAvailability {
                match self {
                    $(Rule::$variant => FixAvailability::$fix,)*
                }
            }

            // The Python version a rule needs, which a file's target below it leaves the rule out for.
            fn minimum_version(self) -> Option<PythonVersion> {
                match self {
                    $(Rule::$variant => Option::<PythonVersion>::None $(.or(Some(PythonVersion::$min)))?,)*
                }
            }

            pub(crate) fn check_module(self, checker: &Checker) {
                match self {
                    $(Rule::$variant => { $(hook!(module, $hook, $family::$check(checker));)* })*
                }
            }

            // The facts the rule reads, which the run collects only when a rule of the run reads them.
            pub(crate) const fn reads(self) -> &'static [Read] {
                match self {
                    $(Rule::$variant => &[$($(read!($read)),*)?],)*
                }
            }

            pub(crate) const fn skips(self) -> &'static [Skip] {
                match self {
                    $(Rule::$variant => &[$($(skip!($skip)),*)?],)*
                }
            }

            pub(crate) fn reads_other_files(self) -> bool {
                self.reads().contains(&Read::CrossFile)
            }

            // A cross-file rule reads the imports the run's doctests make, which the index resolves as imports.
            pub(crate) fn fact_parts(self) -> Parts {
                self.reads().iter().fold(Parts::default(), |parts, read| match read {
                    Read::Part(part) => parts.with(*part),
                    Read::CrossFile => parts.with(Parts::DOCTESTS),
                    _ => parts,
                })
            }

            pub(crate) fn walks(self) -> bool {
                match self {
                    $(Rule::$variant => false $(|| hook!(walks, $hook))*,)*
                }
            }
        }

        pub(crate) fn statement<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
            $($(if rules.contains(Rule::$variant) {
                hook!(statement, $hook, walker, Rule::$variant, $family::$check(walker, stmt))
            })*)*
        }

        pub(crate) fn expression<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
            $($(if rules.contains(Rule::$variant) {
                hook!(expression, $hook, walker, Rule::$variant, $family::$check(walker, expr))
            })*)*
        }

        pub(crate) fn scope<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
            $($(if rules.contains(Rule::$variant) {
                hook!(scope, $hook, walker, Rule::$variant, $family::$check(walker, scope))
            })*)*
        }

        pub(crate) fn except_handler<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
            $($(if rules.contains(Rule::$variant) {
                hook!(except_handler, $hook, walker, Rule::$variant, $family::$check(walker, handler))
            })*)*
        }

        pub(crate) fn pattern<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
            $($(if rules.contains(Rule::$variant) {
                hook!(pattern, $hook, walker, Rule::$variant, $family::$check(walker, pattern))
            })*)*
        }

        pub(crate) fn type_param<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
            $($(if rules.contains(Rule::$variant) {
                hook!(type_param, $hook, walker, Rule::$variant, $family::$check(walker, type_param))
            })*)*
        }

    };
}

// A fact a rule reads beyond its own file's syntax.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Read {
    CrossFile,
    SiblingInit,
    Documentation,
    WorkingDirectory,
    Part(Parts),
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Skip {
    Nested,
}

macro_rules! skip {
    (nested) => {
        Skip::Nested
    };
}

macro_rules! read {
    (cross_file) => {
        Read::CrossFile
    };
    (sibling_init) => {
        Read::SiblingInit
    };
    (docs) => {
        Read::Documentation
    };
    (working_directory) => {
        Read::WorkingDirectory
    };
    ($part:ident) => {
        Read::Part(Parts::$part)
    };
}

macro_rules! hook {
    (module, module, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check($($argument),*)
    };
    (statement, statement, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::statement($($argument),*)
        }
    };
    (expression, expression, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::expression($($argument),*)
        }
    };
    (scope, scope, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::scope($($argument),*)
        }
    };
    (except_handler, except_handler, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::except_handler($($argument),*)
        }
    };
    (pattern, pattern, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::pattern($($argument),*)
        }
    };
    (type_param, type_param, $walker:ident, $rule:expr, $family:ident::$check:ident($($argument:expr),*)) => {
        {
            $walker.checker.enter($rule);
            rules::$family::$check::type_param($($argument),*)
        }
    };
    (walks, module) => {
        false
    };
    (walks, $hook:ident) => {
        true
    };
    ($asked:ident, $hook:ident, $($call:tt)*) => {
        ()
    };
}

rules! {
    SectionComment => ("SMP001", "section-comment", "Comments used as dividers or section headers", Always, comments::section_comment { module }),
    LongParagraph => ("SMP002", "long-paragraph", "Comment paragraphs over the word limit", None, comments::long_paragraph { module }),
    SuppressionWithoutReason => ("SMP003", "suppression-without-reason", "`noqa`, `type: ignore` and `pragma` comments with no reason", None, comments::suppression_without_reason { module }),
    BlanketIgnore => ("SMP004", "blanket-ignore", "Suppression comments that name no error code", None, comments::blanket_ignore { module }),
    RestatingComment => ("SMP005", "restating-comment", "Comments that only repeat the code below", Always, comments::restating_comment { module }),
    StringStatementComment => ("SMP006", "string-statement-comment", "Bare strings used as comments instead of `#` comments", Sometimes, comments::string_statement_comment { module }),
    NumericIgnore => ("SMP008", "numeric-ignore", "Numeric codes in simplint ignore comments", Always, comments::numeric_ignore { module }),
    UnusedIgnore => ("SMP009", "unused-ignore", "Unused selectors in simplint ignore comments", Always, comments::unused_ignore {}),
    SingleLetterName => ("SMP101", "single-letter-name", "Single-letter names outside comprehensions and conventional type parameters", None, naming::single_letter_name { statement, expression, scope, except_handler, pattern, type_param }),
    ModuleVariableCase => ("SMP102", "module-variable-case", "Module variables not `UPPER_CASE`", Sometimes, naming::module_variable_case { module } reads [cross_file, STRING_WORDS, COMPUTED_WRITES, DOCTESTS] skips [nested]),
    MeaninglessName => ("SMP103", "meaningless-name", "Variables, parameters and attributes named with a word that says nothing", None, naming::meaningless_name { expression, scope, except_handler, pattern }),
    CatchAllModule => ("SMP104", "catch-all-module", "Modules and packages named for a grab bag of code", None, naming::catch_all_module { module }),
    EntrypointModuleName => ("SMP105", "entrypoint-module-name", "Entrypoints in `main.py` or `_main.py` instead of `__main__.py`", None, naming::entrypoint_module_name { module }),
    UnguardedEntrypointCall => ("SMP106", "unguarded-entrypoint-call", "A trailing top-level call in a runnable module that runs during import", Sometimes, naming::unguarded_entrypoint_call { module }),
    DefinitionBeforeUse => ("SMP201", "definition-before-use", "Helpers defined above their first caller, or away from it", Sometimes, layout::definition_before_use { module }),
    SingleUserConstant => ("SMP202", "single-user-constant", "Module constants only one function uses", Sometimes, layout::single_user_constant { module } reads [cross_file, STRING_WORDS, COMPUTED_WRITES]),
    ClassMemberOrder => ("SMP203", "class-member-order", "Class members defined out of section order, such as a constructor below a method", Sometimes, layout::class_member_order { module }),
    FutureAnnotations => ("SMP301", "future-annotations", "`from __future__ import annotations` on Python 3.14+", Sometimes, imports::future_annotations { module } since PY314),
    AbsoluteSiblingImport => ("SMP302", "absolute-sibling-import", "Absolute imports from the same package", Always, imports::absolute_sibling_import { module } reads [sibling_init]),
    DependencyImportGuard => ("SMP303", "dependency-import-guard", "`try`/`except ImportError` around a required dependency", Sometimes, imports::dependency_import_guard { module }),
    ModuleAttributeImport => ("SMP304", "module-attribute-import", "Module attributes reached as `module.Name` instead of imported", Sometimes, imports::module_attribute_import { module } reads [cross_file, GLOBAL_NAMES, COMPUTED_WRITES, RUNTIME_IMPORTS]),
    BackportImport => ("SMP305", "up-backport-import", "Imports of backports that the standard library replaces", Sometimes, imports::up_backport_import { module } reads [cross_file]),
    TypeCheckingLazyImport => ("SMP306", "up-type-checking-lazy-import", "`if TYPE_CHECKING:` imports that can be `lazy` imports", Sometimes, imports::up_type_checking_lazy_import { module } since PY315),
    LazyModulesList => ("SMP307", "up-lazy-modules-list", "`__lazy_modules__` lists that `lazy` imports replace", Sometimes, imports::up_lazy_modules_list { module } since PY315),
    ImportCycle => ("SMP308", "import-cycle", "Runtime imports that close a cycle between modules", None, imports::import_cycle { module } reads [cross_file, RUNTIME_IMPORTS, working_directory]),
    PrivateImportPath => ("SMP310", "private-import-path", "Private module objects and imports beyond a direct relative path", None, imports::private_import_path { statement } reads [cross_file]),
    FlattenedPackage => ("SMP311", "flattened-package", "Private modules sharing a name prefix instead of a package", None, imports::flattened_package { module } reads [cross_file]),
    MissingFromAll => ("SMP401", "missing-from-all", "Names other modules import that `__all__` omits", Sometimes, api::missing_from_all { module } reads [cross_file] skips [nested]),
    UnusedExport => ("SMP402", "unused-export", "`__all__` entries no other module imports", Sometimes, api::unused_export { module } reads [cross_file, DOCTESTS] skips [nested]),
    UnexportedPublicName => ("SMP403", "unexported-public-name", "Public functions and classes missing from `__all__`", Sometimes, api::unexported_public_name { module } reads [cross_file, STRING_WORDS, REFLECTED_NAMES, COMPUTED_WRITES, DOCTESTS] skips [nested]),
    ModuleVariablePrefix => ("SMP404", "module-variable-prefix", "Module variables whose leading `_` does not match their visibility", Sometimes, api::module_variable_prefix { module } reads [cross_file, STRING_WORDS, COMPUTED_WRITES, DOCTESTS] skips [nested]),
    AllFormat => ("SMP405", "all-format", "`__all__` not a list with a trailing comma", Always, api::all_format { module }),
    AllNotLast => ("SMP406", "all-not-last", "`__all__` not at the end of the module", Sometimes, api::all_not_last { module }),
    InitModuleContent => ("SMP407", "init-module-content", "`__init__.py` holding code beyond its API", None, api::init_module_content { module } reads [cross_file]),
    TestOnlyCode => ("SMP408", "test-only-code", "Module-level functions and classes only test modules use", None, api::test_only_code { module } reads [cross_file, docs]),
    UnusedDefinition => ("SMP409", "unused-definition", "Functions, classes, methods and class attributes nothing in the run uses", None, api::unused_definition { module } reads [cross_file, COMPUTED_READS, DOCTESTS, docs]),
    TestOnlyParameter => ("SMP410", "test-only-parameter", "Parameters only test calls pass, which production calls leave at their default", None, api::test_only_parameter { module } reads [cross_file, CALL_CONTEXTS, DOCTESTS, docs]),
    UnusedReturnValue => ("SMP411", "unused-return-value", "Return values no production call reads", None, api::unused_return_value { module } reads [cross_file, CALL_CONTEXTS, DOCTESTS, docs]),
    ForwardingFunction => ("SMP501", "forwarding-function", "Private functions that only forward to another call", None, logic::forwarding_function { module } reads [cross_file]),
    WalrusCandidate => ("SMP502", "walrus-candidate", "An assignment the next `if` test could absorb with `:=`", Always, logic::walrus_candidate { module } reads [cross_file]),
    StringConcatenation => ("SMP503", "string-concatenation", "`+` between strings", Sometimes, logic::string_concatenation { expression } reads [cross_file]),
    SingleUseVariable => ("SMP504", "single-use-variable", "Variables read once, right below their assignment", Always, logic::single_use_variable { module }),
    SingleCallerHelper => ("SMP505", "single-caller-helper", "Private helpers with one short call site, which inlining would keep short", None, logic::single_caller_helper { module } reads [cross_file]),
    UnusedDefault => ("SMP506", "unused-default", "Parameter defaults that every call passes explicitly", Sometimes, logic::unused_default { module } reads [cross_file, docs]),
    ErrorMessageMatching => ("SMP507", "error-message-matching", "Exceptions told apart by their message text", None, logic::error_message_matching { except_handler } reads [cross_file]),
    ComprehensionUnpacking => ("SMP508", "up-comprehension-unpacking", "Nested `for` clauses that only unpack their iterable into the element", Always, logic::up_comprehension_unpacking { expression } since PY315),
    TracebackExceptionArguments => ("SMP509", "up-traceback-exception-arguments", "`traceback` calls that pass an exception's type and traceback next to it", Always, logic::up_traceback_exception_arguments { expression } since PY310),
    ExcInfoException => ("SMP510", "up-exc-info-exception", "`sys.exc_info()` reads of the exception being handled", Always, logic::up_exc_info_exception { expression } since PY311),
    ObjectSentinel => ("SMP511", "up-object-sentinel", "`object()` placeholders `sentinel` would name", Always, logic::up_object_sentinel { module } since PY315),
    ContextVarTokenReset => ("SMP512", "up-context-var-token-reset", "`ContextVar.set()` tokens reset in a `finally` block", Sometimes, logic::up_context_var_token_reset { module } since PY314 reads [cross_file]),
    ChdirRestore => ("SMP513", "up-chdir-restore", "Working directories restored in a `finally` block", Sometimes, logic::up_chdir_restore { module } since PY311 reads [cross_file]),
    StrptimeDate => ("SMP514", "up-strptime-date", "`datetime.strptime()` results narrowed to a `date` or `time`", Sometimes, logic::up_strptime_date { expression } since PY314),
    EventLoopInCoroutine => ("SMP515", "up-event-loop-in-coroutine", "`asyncio.get_event_loop()` calls in coroutines", Sometimes, logic::up_event_loop_in_coroutine { expression }),
    AsyncioIsCoroutineFunction => ("SMP516", "up-asyncio-iscoroutinefunction", "`asyncio.iscoroutinefunction()` calls", Sometimes, logic::up_asyncio_iscoroutinefunction { expression }),
    CodecsOpen => ("SMP517", "up-codecs-open", "`codecs.open()` calls", Sometimes, logic::up_codecs_open { expression }),
    ReErrorAlias => ("SMP518", "up-re-error-alias", "`re.error` references", Always, logic::up_re_error_alias { expression } since PY313),
    ArgparseSuggestOnError => ("SMP519", "up-argparse-suggest-on-error", "`suggest_on_error=True` in `ArgumentParser()` calls", Sometimes, logic::up_argparse_suggest_on_error { expression } since PY315),
    RemovedAstNode => ("SMP520", "up-removed-ast-node", "`ast` node aliases and visitor methods Python 3.14 removed", Sometimes, logic::up_removed_ast_node { expression, statement }),
    Utf8EncodingDefault => ("SMP521", "up-utf8-encoding-default", "UTF-8 `encoding` arguments that Python 3.15 makes the default", Sometimes, logic::up_utf8_encoding_default { expression } since PY315),
    RePrefixMatch => ("SMP522", "up-re-prefixmatch", "`re.match()` calls that `re.prefixmatch()` names", Always, logic::up_re_prefixmatch { expression } since PY315),
    CompressionModules => ("SMP523", "up-compression-modules", "`gzip`, `bz2`, `lzma` and `zlib` imports that `compression` names", Sometimes, logic::up_compression_modules { module } since PY314),
    WarningsDeprecated => ("SMP524", "up-warnings-deprecated", "Functions and classes that warn with `DeprecationWarning` where `@warnings.deprecated` marks them", Sometimes, logic::up_warnings_deprecated { statement } since PY313),
    DeprecatedStdlibAlias => ("SMP525", "up-deprecated-stdlib-alias", "Deprecated standard library names that have a replacement", Sometimes, logic::up_deprecated_stdlib_alias { expression }),
    EarlyAssignment => ("SMP526", "early-assignment", "Constants assigned well above their first use", Sometimes, logic::early_assignment { module }),
    SuperForwardingMethod => ("SMP527", "super-forwarding-method", "Methods that only return the parent's method called with their own arguments", Sometimes, logic::super_forwarding_method { module }),
    LongTupleReturn => ("SMP528", "long-tuple-return", "Functions returning tuples too long to unpack by position", None, logic::long_tuple_return { statement }),
    CognitiveComplexity => ("SMP529", "cognitive-complexity", "Functions whose cognitive complexity is over the limit", None, logic::cognitive_complexity { module }),
    DuplicateCode => ("SMP530", "duplicate-code", "Statements that another module repeats", None, logic::duplicate_code { module } reads [cross_file, STATEMENTS, working_directory]),
    CollectionBuiltByStatements => ("SMP531", "collection-built-by-statements", "Empty collections filled by the statements right after them", Sometimes, logic::collection_built_by_statements { module }),
    UnnamedRegexGroup => ("SMP532", "unnamed-regex-group", "Regular expressions with unnamed capture groups", Sometimes, logic::unnamed_regex_group { module } reads [cross_file]),
    UnexplainedRegex => ("SMP533", "unexplained-regex", "Regular expressions with four or more parts and no comments", None, logic::unexplained_regex { module }),
    InlineRegex => ("SMP534", "inline-regex", "Regular expressions compiled inside a function", Sometimes, logic::inline_regex { module }),
    MissingFinal => ("SMP601", "missing-final", "Module constants not annotated `Final`", Sometimes, typing::missing_final { module } reads [cross_file, COMPUTED_WRITES]),
    ImpreciseAnnotation => ("SMP602", "imprecise-annotation", "`object` and `Any` in annotations", None, typing::imprecise_annotation { statement }),
    CallableEllipsis => ("SMP603", "callable-ellipsis", "`Callable[..., T]`", None, typing::callable_ellipsis { expression }),
    VarsCall => ("SMP604", "vars-call", "`vars()` calls", None, typing::vars_call { expression }),
    DataclassFrozen => ("SMP701", "dataclass-frozen", "Dataclasses without `frozen`", Always, classes::dataclass_frozen { module } reads [cross_file]),
    DataclassSlots => ("SMP702", "dataclass-slots", "Dataclasses without `slots`", Always, classes::dataclass_slots { module } reads [cross_file]),
    DataclassKwOnly => ("SMP703", "dataclass-kw-only", "Dataclasses without `kw_only`", Always, classes::dataclass_kw_only { module } reads [cross_file]),
    IndistinctErrorClass => ("SMP704", "indistinct-error-class", "Exception classes no handler catches by name", None, classes::indistinct_error_class { module } reads [cross_file, CAUGHT, docs]),
    TestClass => ("SMP801", "test-class", "Test classes", Sometimes, testing::test_class { module } reads [cross_file]),
    ParametrizeWithoutId => ("SMP802", "parametrize-without-id", "`parametrize` rows without `pytest.param(..., id=...)`", Sometimes, testing::parametrize_without_id { module }),
    ConditionalInTest => ("SMP803", "conditional-in-test", "`if`, `match` and conditional expressions in tests", None, testing::conditional_in_test { module }),
    ConstantAssert => ("SMP804", "constant-assert", "Assertions on a truthy constant", Sometimes, testing::constant_assert { module }),
    MockAttributeAssignment => ("SMP805", "mock-attribute-assignment", "Mock attributes assigned after construction", Sometimes, testing::mock_attribute_assignment { module }),
    MockWithoutSpec => ("SMP806", "mock-without-spec", "Mocks and patches without a spec", Sometimes, testing::mock_without_spec { expression }),
    SplitCompositeAssert => ("SMP807", "split-composite-assert", "Consecutive asserts comparing fields of one object", Sometimes, testing::split_composite_assert { module }),
    SleepInTest => ("SMP808", "sleep-in-test", "`time.sleep` and `asyncio.sleep` calls in tests", None, testing::sleep_in_test { module }),
    StdlibMock => ("SMP809", "stdlib-mock", "`unittest.mock` imports in projects that depend on pytest-mock", None, testing::stdlib_mock { module }),
    UnittestEnterContext => ("SMP812", "up-unittest-enter-context", "Patches started and stopped by hand in `TestCase` methods", Sometimes, testing::up_unittest_enter_context { module } since PY311),
    LoopInTest => ("SMP813", "loop-in-test", "Loops in tests that assert over a table of cases", None, testing::loop_in_test { module }),
    MockAssertionTypo => ("SMP814", "mock-assertion-typo", "Mock assertion methods misspelled or missing their `assert_` prefix", Always, testing::mock_assertion_typo { module } reads [cross_file]),
    UnusedFixture => ("SMP815", "unused-fixture", "Pytest fixtures no test or fixture requests", None, testing::unused_fixture { module } reads [cross_file, FIXTURES]),
    FunctionDocstring => ("SMP901", "function-docstring", "Docstrings on internal functions and classes", Sometimes, docstrings::function_docstring { module } reads [cross_file, docs] skips [nested]),
    TestModuleDocstring => ("SMP902", "test-module-docstring", "Docstrings on test modules", Sometimes, docstrings::test_module_docstring { module } skips [nested]),
    UndocumentedPublicApi => ("SMP903", "undocumented-public-api", "Public functions, classes, methods and variables without documentation", None, docstrings::undocumented_public_api { module } reads [cross_file, working_directory] skips [nested]),
    LongDocstringParagraph => ("SMP904", "long-docstring-paragraph", "Docstring paragraphs over the word limit", None, docstrings::long_docstring_paragraph { module } skips [nested]),
    RstFieldSyntax => ("SMP910", "rst-field-syntax", "Malformed or misplaced reStructuredText docstring fields", Sometimes, docstrings::rst_field_syntax { module } reads [docs] skips [nested]),
    RstInlineMarkup => ("SMP911", "rst-inline-markup", "Roles without backticks and unclosed inline literals", Sometimes, docstrings::rst_inline_markup { module } reads [docs] skips [nested]),
    RstTypeField => ("SMP912", "rst-type-field", "Types in docstrings, which belong in annotations", Sometimes, docstrings::rst_type_field { module } reads [docs] skips [nested]),
    RstRestatingField => ("SMP913", "rst-restating-field", "Field descriptions that only repeat the name", None, docstrings::rst_restating_field { module } reads [docs] skips [nested]),
    RstEmptyField => ("SMP914", "rst-empty-field", "Fields with no description", None, docstrings::rst_empty_field { module } reads [docs] skips [nested]),
    RstPrivateField => ("SMP915", "rst-private-field", "Public docstrings documenting internal names", Sometimes, docstrings::rst_private_field { module } reads [cross_file, docs] skips [nested]),
    RstMissingParam => ("SMP916", "rst-missing-param", "Public API docstrings that leave a parameter undocumented", None, docstrings::rst_missing_param { module } reads [cross_file, docs] skips [nested]),
    RstUnknownParam => ("SMP917", "rst-unknown-param", "`:param:` and `:type:` fields for parameters the signature lacks", Sometimes, docstrings::rst_unknown_param { module } reads [docs] skips [nested]),
    RstDuplicateParam => ("SMP918", "rst-duplicate-param", "Constructor parameters documented in both the class and `__init__` docstrings", Sometimes, docstrings::rst_duplicate_param { module } reads [cross_file, docs] skips [nested]),
    RstParamOrder => ("SMP919", "rst-param-order", "`:param:` fields out of the signature's order", None, docstrings::rst_param_order { module } reads [docs] skips [nested]),
    RstMissingAttribute => ("SMP920", "rst-missing-attribute", "Public classes whose docstring leaves an attribute undocumented", None, docstrings::rst_missing_attribute { module } reads [cross_file, docs] skips [nested]),
    RstUnknownAttribute => ("SMP921", "rst-unknown-attribute", "`:ivar:`, `:cvar:` and `:var:` fields for attributes the class lacks", Sometimes, docstrings::rst_unknown_attribute { module } reads [docs] skips [nested]),
    RstAttributeKind => ("SMP922", "rst-attribute-kind", "`:ivar:` on class variables and `:cvar:` on instance attributes", Sometimes, docstrings::rst_attribute_kind { module } reads [docs] skips [nested]),
    RstReturns => ("SMP923", "rst-returns", "Missing, extraneous or misused `:returns:` and `:yields:` fields", Sometimes, docstrings::rst_returns { module } reads [cross_file, docs] skips [nested]),
    RstMissingRaises => ("SMP924", "rst-missing-raises", "Public API docstrings that leave a raised exception undocumented", None, docstrings::rst_missing_raises { module } reads [cross_file, docs] skips [nested]),
}

// The simplint rule behind a diagnostic; `None` for Ruff's own diagnostics, such as a syntax error.
pub fn rule_of(diagnostic: &Diagnostic) -> Option<Rule> {
    diagnostic.secondary_code().and_then(|code| Rule::from_code(code.as_str()))
}

#[derive(Clone, Copy, Default, CacheKey)]
pub struct RuleSet(u128);

const _: () = assert!(Rule::ALL.len() <= 128, "RuleSet holds one bit per rule in a u128");

impl RuleSet {
    // A selector's rules and how specific it is, ordered as Ruff's `Specificity`: `ALL`, then code prefixes by length,
    // then a single rule by code or name. `None` when it selects nothing.
    pub fn from_selector(selector: &str) -> Option<(Self, usize)> {
        if selector == "ALL" {
            return Some((Self::all(), 0));
        }
        if let Some(rule) = Rule::from_code_or_name(selector) {
            return Some((Self::from_rules([rule]), usize::MAX));
        }
        let set = Self::from_rules(Rule::ALL.iter().copied().filter(|rule| rule.code().starts_with(selector)));
        (!selector.is_empty() && set.0 != 0).then_some((set, selector.len()))
    }

    pub fn all() -> Self {
        Self::from_rules(Rule::ALL.iter().copied())
    }

    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Self {
        Self(rules.into_iter().fold(0, |bits, rule| bits | 1 << rule as u128))
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn for_target(self, target: PythonVersion) -> Self {
        let newer =
            Rule::ALL.iter().copied().filter(|rule| rule.minimum_version().is_some_and(|minimum| minimum > target));
        self.subtract(Self::from_rules(newer))
    }

    pub fn subtract(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Rule> {
        Rule::ALL.iter().copied().filter(move |rule| self.contains(*rule))
    }

    pub fn contains(self, rule: Rule) -> bool {
        self.0 & 1 << rule as u128 != 0
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    // The rules that read the project index, directly or through the public API, the rst targets' publicness, string
    // kinds or the rename fix, so another file of the run can change what they report or fix.
    pub fn cross_file() -> Self {
        static CROSS_FILE: LazyLock<RuleSet> = LazyLock::new(|| RuleSet::reading(Read::CrossFile));
        *CROSS_FILE
    }

    fn reading(read: Read) -> Self {
        Self::from_rules(Rule::ALL.iter().copied().filter(|rule| rule.reads().contains(&read)))
    }

    pub fn sibling_init() -> Self {
        static SIBLING_INIT: LazyLock<RuleSet> = LazyLock::new(|| RuleSet::reading(Read::SiblingInit));
        *SIBLING_INIT
    }

    // The rules a nested project leaves out, as its files are test input whose docstrings and names are not judged.
    pub fn skipped_in_nested() -> Self {
        static SKIPPED_IN_NESTED: LazyLock<RuleSet> = LazyLock::new(|| {
            RuleSet::from_rules(Rule::ALL.iter().copied().filter(|rule| rule.skips().contains(&Skip::Nested)))
        });
        *SKIPPED_IN_NESTED
    }

    pub fn working_directory() -> Self {
        static WORKING_DIRECTORY: LazyLock<RuleSet> = LazyLock::new(|| RuleSet::reading(Read::WorkingDirectory));
        *WORKING_DIRECTORY
    }

    pub fn documentation() -> Self {
        static DOCUMENTATION: LazyLock<RuleSet> = LazyLock::new(|| RuleSet::reading(Read::Documentation));
        *DOCUMENTATION
    }
}

// Each group owns the code digit in this order: `SMP0` is the first group, `SMP9` the last. The short title fits the
// documentation's sidebar.
pub const GROUPS: [(&str, &str, &str); 10] = [
    ("comments", "Comments", "Comments"),
    ("naming", "Naming", "Naming"),
    ("layout", "Module layout", "Module layout"),
    ("imports", "Imports and dependencies", "Imports"),
    ("api", "Public API", "Public API"),
    ("logic", "Logic", "Logic"),
    ("typing", "Typing", "Typing"),
    ("classes", "Classes", "Classes"),
    ("testing", "Tests", "Tests"),
    ("docstrings", "Docstrings", "Docstrings"),
];

impl Rule {
    pub fn group(self) -> &'static str {
        self.group_entry().1
    }

    // `--select SMP6` picks rules by this digit, so the listing groups them by it too.
    fn group_entry(self) -> (&'static str, &'static str, &'static str) {
        GROUPS[usize::from(self.code().as_bytes()["SMP".len()] - b'0')]
    }

    pub fn short_group(self) -> &'static str {
        self.group_entry().2
    }
}

#[cfg(test)]
mod tests {
    use super::Rule;

    #[test]
    fn code_digit_names_the_family_group() {
        let misplaced: Vec<_> = Rule::ALL
            .iter()
            .filter(|rule| rule.group_entry().0 != rule.family())
            .map(|rule| (rule.code(), rule.family(), rule.group_entry().0))
            .collect();
        assert_eq!(misplaced, Vec::new());
    }
}
