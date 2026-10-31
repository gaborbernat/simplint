use ruff_db::diagnostic::Diagnostic;
use ruff_linter::FixAvailability;
use ruff_macros::CacheKey;
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt, TypeParam};

use crate::lint::checker::Checker;
use crate::lint::walker::{Scope, Walker};
use crate::rules;

// Each rule lists its hooks: `module` runs the rule's function once over the whole module; every other hook names the
// kind of node the walker hands to the function of that name in the rule's module.
macro_rules! rules {
    ($($variant:ident => (
        $code:literal, $name:literal, $summary:literal, $fix:ident, $family:ident::$check:ident { $($hook:ident),+ }
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

            pub fn fix_availability(self) -> FixAvailability {
                match self {
                    $(Rule::$variant => FixAvailability::$fix,)*
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
    FunctionDocstring => ("SMP002", "function-docstring", "Docstrings on internal functions and classes", Sometimes, docstrings::function_docstring { module }),
    MultilineModuleDocstring => ("SMP003", "multiline-module-docstring", "Module docstrings longer than one line", None, docstrings::multiline_module_docstring { module }),
    TestModuleDocstring => ("SMP004", "test-module-docstring", "Docstrings on test modules", Sometimes, docstrings::test_module_docstring { module }),
    LongParagraph => ("SMP005", "long-paragraph", "Comment or docstring paragraphs over 100 words", None, comments::long_paragraph { module }),
    SingleLetterName => ("SMP010", "single-letter-name", "Single-letter names outside loop targets and comprehensions", None, naming::single_letter_name { statement, expression, scope, except_handler, pattern, type_param }),
    DefinitionBeforeUse => ("SMP020", "definition-before-use", "Helpers defined above their first caller, or away from it", Sometimes, structure::definition_before_use { module }),
    ForwardingFunction => ("SMP030", "forwarding-function", "Private functions that only forward to another call", None, structure::forwarding_function { module }),
    DependencyImportGuard => ("SMP040", "dependency-import-guard", "`try`/`except ImportError` around a required dependency", Sometimes, imports::dependency_import_guard { module }),
    SingleUserConstant => ("SMP050", "single-user-constant", "Module constants only one function uses", Sometimes, structure::single_user_constant { module }),
    SuppressionWithoutReason => ("SMP060", "suppression-without-reason", "`noqa`, `type: ignore` and `pragma` comments with no reason", None, comments::suppression_without_reason { module }),
    WalrusCandidate => ("SMP100", "walrus-candidate", "An assignment the next `if` test could absorb with `:=`", Always, structure::walrus_candidate { module }),
    MissingFinal => ("SMP101", "missing-final", "Module constants not annotated `Final`", Sometimes, typing::missing_final { module }),
    ModuleVariableCase => ("SMP102", "module-variable-case", "Module variables not `UPPER_CASE`, or wrongly `_`-prefixed", Sometimes, naming::module_variable_case { module }),
    UnexportedPublicName => ("SMP103", "unexported-public-name", "Public functions and classes missing from `__all__`", Sometimes, naming::unexported_public_name { module }),
    AllNotLast => ("SMP104", "all-not-last", "`__all__` not at the end of the module", Sometimes, exports::all_not_last { module }),
    CallableEllipsis => ("SMP105", "callable-ellipsis", "`Callable[..., T]`", None, typing::callable_ellipsis { expression }),
    ObjectAnnotation => ("SMP106", "object-annotation", "`object` annotations outside `TypeIs` guards and dunders", None, typing::object_annotation { statement }),
    VarsCall => ("SMP107", "vars-call", "`vars()` calls", None, typing::vars_call { expression }),
    FutureAnnotations => ("SMP108", "future-annotations", "`from __future__ import annotations` on Python 3.14+", Sometimes, imports::future_annotations { module }),
    InitModuleContent => ("SMP109", "init-module-content", "`__init__.py` holding more than a docstring", None, imports::init_module_content { module }),
    AbsoluteSiblingImport => ("SMP110", "absolute-sibling-import", "Absolute imports from the same package", Always, imports::absolute_sibling_import { module }),
    DataclassOptions => ("SMP111", "dataclass-options", "Dataclasses without `frozen` and `slots`", Always, dataclasses::dataclass_options { module }),
    StringConcatenation => ("SMP112", "string-concatenation", "`+` between strings", Sometimes, strings::string_concatenation { expression }),
    MissingFromAll => ("SMP113", "missing-from-all", "Names other modules import that `__all__` omits", Sometimes, exports::missing_from_all { module }),
    DataclassKwOnly => ("SMP114", "dataclass-kw-only", "Dataclasses without `kw_only`", Always, dataclasses::dataclass_kw_only { module }),
    AllFormat => ("SMP115", "all-format", "`__all__` not a list with a trailing comma", Always, exports::all_format { module }),
    UndocumentedPublicApi => ("SMP116", "undocumented-public-api", "Exported functions, classes, methods and variables without documentation", None, docstrings::undocumented_public_api { module }),
    TestClass => ("SMP200", "test-class", "Test classes", Sometimes, testing::test_class { module }),
    ParametrizeWithoutId => ("SMP201", "parametrize-without-id", "`parametrize` rows without `pytest.param(..., id=...)`", None, testing::parametrize_without_id { module }),
    ConditionalInTest => ("SMP202", "conditional-in-test", "`if`, `match` and conditional expressions in tests", None, testing::conditional_in_test { module }),
    MockAttributeAssignment => ("SMP203", "mock-attribute-assignment", "Mock attributes assigned after construction", Sometimes, testing::mock_attribute_assignment { module }),
    MockWithoutSpec => ("SMP204", "mock-without-spec", "Mocks and patches without a spec", None, testing::mock_without_spec { expression }),
    ConstantAssert => ("SMP205", "constant-assert", "Assertions on a truthy constant", Sometimes, testing::constant_assert { module }),
}

// The simplint rule behind a diagnostic; `None` for Ruff's own diagnostics, such as a syntax error.
pub fn rule_of(diagnostic: &Diagnostic) -> Option<Rule> {
    diagnostic.secondary_code().and_then(|code| Rule::from_code(code.as_str()))
}

// One bit per rule.
#[derive(Clone, Copy, Default, CacheKey)]
pub struct RuleSet(u64);

const _: () = assert!(Rule::ALL.len() <= 64, "RuleSet holds one bit per rule in a u64");

impl RuleSet {
    pub fn all() -> Self {
        Self::from_rules(Rule::ALL.iter().copied())
    }

    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Self {
        Self(rules.into_iter().fold(0, |bits, rule| bits | 1 << rule as u64))
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

    pub fn contains(self, rule: Rule) -> bool {
        self.0 & 1 << rule as u64 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Rule> {
        Rule::ALL.iter().copied().filter(move |rule| self.contains(*rule))
    }
}

// The rule groups `simplint linter` lists, as `ruff linter` lists the linters Ruff ports.
pub const GROUPS: [(&str, &str); 3] =
    [("SMP0", "Comments, docstrings and structure"), ("SMP1", "Modules, typing and exports"), ("SMP2", "Tests")];

impl Rule {
    pub fn group(self) -> &'static str {
        GROUPS.iter().find(|(prefix, _)| self.code().starts_with(prefix)).map_or("", |(_, name)| name)
    }
}
