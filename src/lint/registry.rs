use ruff_db::diagnostic::Diagnostic;
use ruff_linter::FixAvailability;
use ruff_macros::CacheKey;
use ruff_python_ast::{self as ast, Expr, Pattern, PythonVersion, Stmt, TypeParam};

use crate::lint::checker::Checker;
use crate::lint::walker::{Scope, Walker};
use crate::rules;

// Each rule lists its hooks: `module` runs the rule's function once over the whole module; every other hook names the
// kind of node the walker hands to the function of that name in the rule's module.
macro_rules! rules {
    ($($variant:ident => (
        $code:literal, $name:literal, $summary:literal, $fix:ident, $family:ident::$check:ident { $($hook:ident),+ }
        $(since $min:ident)?
    ),)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
                Self::ALL.iter().copied().find(|rule| rule.code() == code)
            }

            pub fn from_code_or_name(value: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|rule| rule.code() == value || rule.name() == value)
            }

            pub fn url(self) -> String {
                format!("https://simplint.readthedocs.io/en/latest/_generated/rules/{}.html", self.code())
            }

            // The directory under `src/rules` the rule lives in, which names its group.
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
            pub fn minimum_version(self) -> Option<PythonVersion> {
                match self {
                    $(Rule::$variant => Option::<PythonVersion>::None $(.or(Some(PythonVersion::$min)))?,)*
                }
            }

            pub(crate) fn check_module(self, checker: &Checker) {
                match self {
                    $(Rule::$variant => { $(hook!(module, $hook, $family::$check(checker));)+ })*
                }
            }

            pub(crate) fn walks(self) -> bool {
                match self {
                    $(Rule::$variant => false $(|| hook!(walks, $hook))+,)*
                }
            }
        }

        pub(crate) fn statement<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
            $($(if rules.contains(Rule::$variant) { hook!(statement, $hook, $family::$check(walker, stmt)) })+)*
        }

        pub(crate) fn expression<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
            $($(if rules.contains(Rule::$variant) { hook!(expression, $hook, $family::$check(walker, expr)) })+)*
        }

        pub(crate) fn scope<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
            $($(if rules.contains(Rule::$variant) { hook!(scope, $hook, $family::$check(walker, scope)) })+)*
        }

        pub(crate) fn except_handler<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
            $($(if rules.contains(Rule::$variant) { hook!(except_handler, $hook, $family::$check(walker, handler)) })+)*
        }

        pub(crate) fn pattern<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
            $($(if rules.contains(Rule::$variant) { hook!(pattern, $hook, $family::$check(walker, pattern)) })+)*
        }

        pub(crate) fn type_param<'a>(rules: RuleSet, walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
            $($(if rules.contains(Rule::$variant) { hook!(type_param, $hook, $family::$check(walker, type_param)) })+)*
        }
    };
}

// A rule's code for one hook: its call when the hook is the one asked for, nothing otherwise. `walks` asks whether the
// hook needs the walker.
macro_rules! hook {
    (module, module, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check($($argument),*)
    };
    (statement, statement, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::statement($($argument),*)
    };
    (expression, expression, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::expression($($argument),*)
    };
    (scope, scope, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::scope($($argument),*)
    };
    (except_handler, except_handler, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::except_handler($($argument),*)
    };
    (pattern, pattern, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::pattern($($argument),*)
    };
    (type_param, type_param, $family:ident::$check:ident($($argument:expr),*)) => {
        rules::$family::$check::type_param($($argument),*)
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
    SingleLetterName => ("SMP101", "single-letter-name", "Single-letter names outside comprehensions", None, naming::single_letter_name { statement, expression, scope, except_handler, pattern, type_param }),
    ModuleVariableCase => ("SMP102", "module-variable-case", "Module variables not `UPPER_CASE`", Sometimes, naming::module_variable_case { module }),
    DefinitionBeforeUse => ("SMP201", "definition-before-use", "Helpers defined above their first caller, or away from it", Sometimes, layout::definition_before_use { module }),
    SingleUserConstant => ("SMP202", "single-user-constant", "Module constants only one function uses", Sometimes, layout::single_user_constant { module }),
    ConstructorAfterMethod => ("SMP203", "constructor-after-method", "Constructors defined below other methods", Sometimes, layout::constructor_after_method { module }),
    FutureAnnotations => ("SMP301", "future-annotations", "`from __future__ import annotations` on Python 3.14+", Sometimes, imports::future_annotations { module } since PY314),
    AbsoluteSiblingImport => ("SMP302", "absolute-sibling-import", "Absolute imports from the same package", Always, imports::absolute_sibling_import { module }),
    DependencyImportGuard => ("SMP303", "dependency-import-guard", "`try`/`except ImportError` around a required dependency", Sometimes, imports::dependency_import_guard { module }),
    ModuleAttributeImport => ("SMP304", "module-attribute-import", "Module attributes reached as `module.Name` instead of imported", Sometimes, imports::module_attribute_import { module }),
    BackportImport => ("SMP305", "up-backport-import", "Imports of backports that the standard library replaces", Sometimes, imports::up_backport_import { module }),
    TypeCheckingLazyImport => ("SMP306", "up-type-checking-lazy-import", "`if TYPE_CHECKING:` imports that can be `lazy` imports", Sometimes, imports::up_type_checking_lazy_import { module } since PY315),
    LazyModulesList => ("SMP307", "up-lazy-modules-list", "`__lazy_modules__` lists that `lazy` imports replace", Sometimes, imports::up_lazy_modules_list { module } since PY315),
    MissingFromAll => ("SMP401", "missing-from-all", "Names other modules import that `__all__` omits", Sometimes, api::missing_from_all { module }),
    UnusedExport => ("SMP402", "unused-export", "`__all__` entries no other module imports", Sometimes, api::unused_export { module }),
    UnexportedPublicName => ("SMP403", "unexported-public-name", "Public functions and classes missing from `__all__`", Sometimes, api::unexported_public_name { module }),
    ModuleVariablePrefix => ("SMP404", "module-variable-prefix", "Module variables whose leading `_` does not match their visibility", Sometimes, api::module_variable_prefix { module }),
    AllFormat => ("SMP405", "all-format", "`__all__` not a list with a trailing comma", Always, api::all_format { module }),
    AllNotLast => ("SMP406", "all-not-last", "`__all__` not at the end of the module", Sometimes, api::all_not_last { module }),
    InitModuleContent => ("SMP407", "init-module-content", "`__init__.py` holding more than a docstring", None, api::init_module_content { module }),
    TestOnlyCode => ("SMP408", "test-only-code", "Module-level functions and classes only test modules use", None, api::test_only_code { module }),
    ForwardingFunction => ("SMP501", "forwarding-function", "Private functions that only forward to another call", None, logic::forwarding_function { module }),
    WalrusCandidate => ("SMP502", "walrus-candidate", "An assignment the next `if` test could absorb with `:=`", Always, logic::walrus_candidate { module }),
    StringConcatenation => ("SMP503", "string-concatenation", "`+` between strings", Sometimes, logic::string_concatenation { expression }),
    SingleUseVariable => ("SMP504", "single-use-variable", "Variables read once, right below their assignment", Always, logic::single_use_variable { module }),
    SingleCallerHelper => ("SMP505", "single-caller-helper", "Private helpers with one short call site, which inlining would keep short", None, logic::single_caller_helper { module }),
    UnusedDefault => ("SMP506", "unused-default", "Parameter defaults that every call passes explicitly", Sometimes, logic::unused_default { module }),
    ErrorMessageMatching => ("SMP507", "error-message-matching", "Exceptions told apart by their message text", None, logic::error_message_matching { except_handler }),
    ComprehensionUnpacking => ("SMP508", "up-comprehension-unpacking", "Nested `for` clauses that only unpack their iterable into the element", Always, logic::up_comprehension_unpacking { expression } since PY315),
    TracebackExceptionArguments => ("SMP509", "up-traceback-exception-arguments", "`traceback` calls that pass an exception's type and traceback next to it", Always, logic::up_traceback_exception_arguments { expression } since PY310),
    ExcInfoException => ("SMP510", "up-exc-info-exception", "`sys.exc_info()` reads of the exception being handled", Always, logic::up_exc_info_exception { expression } since PY311),
    ObjectSentinel => ("SMP511", "up-object-sentinel", "`object()` placeholders `sentinel` would name", Always, logic::up_object_sentinel { module } since PY315),
    ContextVarTokenReset => ("SMP512", "up-context-var-token-reset", "`ContextVar.set()` tokens reset in a `finally` block", Sometimes, logic::up_context_var_token_reset { module } since PY314),
    ChdirRestore => ("SMP513", "up-chdir-restore", "Working directories restored in a `finally` block", Sometimes, logic::up_chdir_restore { module } since PY311),
    StrptimeDate => ("SMP514", "up-strptime-date", "`datetime.strptime()` results narrowed to a `date` or `time`", Sometimes, logic::up_strptime_date { expression } since PY314),
    EventLoopInCoroutine => ("SMP515", "up-event-loop-in-coroutine", "`asyncio.get_event_loop()` calls in coroutines", Sometimes, logic::up_event_loop_in_coroutine { expression }),
    AsyncioIsCoroutineFunction => ("SMP516", "up-asyncio-iscoroutinefunction", "`asyncio.iscoroutinefunction()` calls", Sometimes, logic::up_asyncio_iscoroutinefunction { expression }),
    CodecsOpen => ("SMP517", "up-codecs-open", "`codecs.open()` calls", Sometimes, logic::up_codecs_open { expression }),
    ReErrorAlias => ("SMP518", "up-re-error-alias", "`re.error` references", Always, logic::up_re_error_alias { expression } since PY313),
    ArgparseSuggestOnError => ("SMP519", "up-argparse-suggest-on-error", "`suggest_on_error=True` in `ArgumentParser()` calls", Sometimes, logic::up_argparse_suggest_on_error { expression } since PY315),
    RemovedAstNode => ("SMP520", "up-removed-ast-node", "`ast` node aliases and visitor methods Python 3.14 removed", Sometimes, logic::up_removed_ast_node { expression, statement }),
    MissingFinal => ("SMP601", "missing-final", "Module constants not annotated `Final`", Sometimes, typing::missing_final { module }),
    ImpreciseAnnotation => ("SMP602", "imprecise-annotation", "`object` and `Any` in annotations", None, typing::imprecise_annotation { statement }),
    CallableEllipsis => ("SMP603", "callable-ellipsis", "`Callable[..., T]`", None, typing::callable_ellipsis { expression }),
    VarsCall => ("SMP604", "vars-call", "`vars()` calls", None, typing::vars_call { expression }),
    DataclassFrozen => ("SMP701", "dataclass-frozen", "Dataclasses without `frozen`", Always, classes::dataclass_frozen { module }),
    DataclassSlots => ("SMP702", "dataclass-slots", "Dataclasses without `slots`", Always, classes::dataclass_slots { module }),
    DataclassKwOnly => ("SMP703", "dataclass-kw-only", "Dataclasses without `kw_only`", Always, classes::dataclass_kw_only { module }),
    IndistinctErrorClass => ("SMP704", "indistinct-error-class", "Exception classes no handler catches by name", None, classes::indistinct_error_class { module }),
    TestClass => ("SMP801", "test-class", "Test classes", Sometimes, testing::test_class { module }),
    ParametrizeWithoutId => ("SMP802", "parametrize-without-id", "`parametrize` rows without `pytest.param(..., id=...)`", None, testing::parametrize_without_id { module }),
    ConditionalInTest => ("SMP803", "conditional-in-test", "`if`, `match` and conditional expressions in tests", None, testing::conditional_in_test { module }),
    ConstantAssert => ("SMP804", "constant-assert", "Assertions on a truthy constant", Sometimes, testing::constant_assert { module }),
    MockAttributeAssignment => ("SMP805", "mock-attribute-assignment", "Mock attributes assigned after construction", Sometimes, testing::mock_attribute_assignment { module }),
    MockWithoutSpec => ("SMP806", "mock-without-spec", "Mocks and patches without a spec", None, testing::mock_without_spec { expression }),
    SplitCompositeAssert => ("SMP807", "split-composite-assert", "Consecutive asserts comparing fields of one object", Sometimes, testing::split_composite_assert { module }),
    SleepInTest => ("SMP808", "sleep-in-test", "`time.sleep` and `asyncio.sleep` calls in tests", None, testing::sleep_in_test { module }),
    StdlibMock => ("SMP809", "stdlib-mock", "`unittest.mock` imports in projects that depend on pytest-mock", None, testing::stdlib_mock { module }),
    UnittestAssertHelper => ("SMP811", "up-unittest-assert-helper", "`assertTrue` and `assertFalse` on checks that `TestCase` has a helper for", Sometimes, testing::up_unittest_assert_helper { expression } since PY314),
    UnittestEnterContext => ("SMP812", "up-unittest-enter-context", "Patches started and stopped by hand in `TestCase` methods", Sometimes, testing::up_unittest_enter_context { module } since PY311),
    FunctionDocstring => ("SMP901", "function-docstring", "Docstrings on internal functions and classes", Sometimes, docstrings::function_docstring { module }),
    TestModuleDocstring => ("SMP902", "test-module-docstring", "Docstrings on test modules", Sometimes, docstrings::test_module_docstring { module }),
    UndocumentedPublicApi => ("SMP903", "undocumented-public-api", "Public functions, classes, methods and variables without documentation", None, docstrings::undocumented_public_api { module }),
    LongDocstringParagraph => ("SMP904", "long-docstring-paragraph", "Docstring paragraphs over the word limit", None, docstrings::long_docstring_paragraph { module }),
    RstFieldSyntax => ("SMP910", "rst-field-syntax", "Malformed or misplaced reStructuredText docstring fields", Sometimes, docstrings::rst_field_syntax { module }),
    RstInlineMarkup => ("SMP911", "rst-inline-markup", "Roles without backticks and unclosed inline literals", Sometimes, docstrings::rst_inline_markup { module }),
    RstTypeField => ("SMP912", "rst-type-field", "Types in docstrings, which belong in annotations", Sometimes, docstrings::rst_type_field { module }),
    RstRestatingField => ("SMP913", "rst-restating-field", "Field descriptions that only repeat the name", None, docstrings::rst_restating_field { module }),
    RstEmptyField => ("SMP914", "rst-empty-field", "Fields with no description", None, docstrings::rst_empty_field { module }),
    RstPrivateField => ("SMP915", "rst-private-field", "Public docstrings documenting internal names", Sometimes, docstrings::rst_private_field { module }),
    RstMissingParam => ("SMP916", "rst-missing-param", "Public API docstrings that leave a parameter undocumented", Sometimes, docstrings::rst_missing_param { module }),
    RstUnknownParam => ("SMP917", "rst-unknown-param", "`:param:` and `:type:` fields for parameters the signature lacks", Sometimes, docstrings::rst_unknown_param { module }),
    RstDuplicateParam => ("SMP918", "rst-duplicate-param", "Constructor parameters documented in both the class and `__init__` docstrings", Sometimes, docstrings::rst_duplicate_param { module }),
    RstParamOrder => ("SMP919", "rst-param-order", "`:param:` fields out of the signature's order", None, docstrings::rst_param_order { module }),
    RstMissingAttribute => ("SMP920", "rst-missing-attribute", "Public classes whose docstring leaves an attribute undocumented", Sometimes, docstrings::rst_missing_attribute { module }),
    RstUnknownAttribute => ("SMP921", "rst-unknown-attribute", "`:ivar:`, `:cvar:` and `:var:` fields for attributes the class lacks", Sometimes, docstrings::rst_unknown_attribute { module }),
    RstAttributeKind => ("SMP922", "rst-attribute-kind", "`:ivar:` on class variables and `:cvar:` on instance attributes", Sometimes, docstrings::rst_attribute_kind { module }),
    RstReturns => ("SMP923", "rst-returns", "Missing, extraneous or misused `:returns:` and `:yields:` fields", Sometimes, docstrings::rst_returns { module }),
    RstMissingRaises => ("SMP924", "rst-missing-raises", "Public API docstrings that leave a raised exception undocumented", Sometimes, docstrings::rst_missing_raises { module }),
}

// The simplint rule behind a diagnostic; `None` for Ruff's own diagnostics, such as a syntax error.
pub fn rule_of(diagnostic: &Diagnostic) -> Option<Rule> {
    diagnostic.secondary_code().and_then(|code| Rule::from_code(code.as_str()))
}

// One bit per rule.
#[derive(Clone, Copy, Default, CacheKey)]
pub struct RuleSet(u128);

const _: () = assert!(Rule::ALL.len() <= 128, "RuleSet holds one bit per rule in a u128");

impl RuleSet {
    pub fn all() -> Self {
        Self::from_rules(Rule::ALL.iter().copied())
    }

    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Self {
        Self(rules.into_iter().fold(0, |bits, rule| bits | 1 << rule as u128))
    }

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

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn subtract(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    // The rules left on for a target Python version: the ones whose minimum version the target reaches.
    pub fn for_target(self, target: PythonVersion) -> Self {
        let newer =
            Rule::ALL.iter().copied().filter(|rule| rule.minimum_version().is_some_and(|minimum| minimum > target));
        self.subtract(Self::from_rules(newer))
    }

    pub fn contains(self, rule: Rule) -> bool {
        self.0 & 1 << rule as u128 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Rule> {
        Rule::ALL.iter().copied().filter(move |rule| self.contains(*rule))
    }
}

// The rule groups `simplint linter` lists, as `ruff linter` lists the linters Ruff ports. Each group owns the code
// digit in this order: `SMP0` is the first group, `SMP9` the last. The short title fits the documentation's sidebar.
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
    fn group_entry(self) -> Option<(&'static str, &'static str, &'static str)> {
        GROUPS.iter().copied().find(|(family, _, _)| *family == self.family())
    }

    pub fn group(self) -> &'static str {
        self.group_entry().map_or("", |(_, name, _)| name)
    }

    pub fn short_group(self) -> &'static str {
        self.group_entry().map_or("", |(_, _, short)| short)
    }

    // The rules that read the project index, directly or through the public API, the rst targets, string kinds or the
    // rename fix, so another file of the run can change what they report or fix. Keep in step with `Checker::project`.
    pub fn reads_other_files(self) -> bool {
        matches!(
            self,
            Rule::ForwardingFunction
                | Rule::SingleUserConstant
                | Rule::WalrusCandidate
                | Rule::SingleCallerHelper
                | Rule::MissingFinal
                | Rule::ModuleVariableCase
                | Rule::ModuleVariablePrefix
                | Rule::UnexportedPublicName
                | Rule::InitModuleContent
                | Rule::DataclassFrozen
                | Rule::DataclassSlots
                | Rule::StringConcatenation
                | Rule::UnusedDefault
                | Rule::MissingFromAll
                | Rule::ModuleAttributeImport
                | Rule::DataclassKwOnly
                | Rule::IndistinctErrorClass
                | Rule::UnusedExport
                | Rule::TestOnlyCode
                | Rule::TestClass
                | Rule::FunctionDocstring
                | Rule::UndocumentedPublicApi
                | Rule::RstFieldSyntax
                | Rule::RstUnknownParam
                | Rule::RstMissingParam
                | Rule::RstParamOrder
                | Rule::RstTypeField
                | Rule::RstReturns
                | Rule::RstMissingRaises
                | Rule::RstMissingAttribute
                | Rule::RstUnknownAttribute
                | Rule::RstDuplicateParam
                | Rule::RstAttributeKind
                | Rule::RstRestatingField
                | Rule::RstEmptyField
                | Rule::RstInlineMarkup
                | Rule::RstPrivateField
        )
    }
}
