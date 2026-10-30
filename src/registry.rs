use anyhow::{Result, bail};

use crate::checker::Checker;
use crate::rules;

macro_rules! rules {
    ($($variant:ident => ($code:literal, $name:literal, $summary:literal, $fix:ident, $check:path),)*) => {
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
                    $(Rule::$variant => include_str!(concat!("rules/docs/", $code, ".md")),)*
                }
            }

            pub fn fix_availability(self) -> FixAvailability {
                match self {
                    $(Rule::$variant => FixAvailability::$fix,)*
                }
            }

            pub(crate) fn check(self, checker: &mut Checker) {
                match self {
                    $(Rule::$variant => $check(checker),)*
                }
            }
        }
    };
}

rules! {
    SectionComment => ("SMP001", "section-comment", "Comments used as dividers or section headers", Always, rules::comments::section_comment),
    FunctionDocstring => ("SMP002", "function-docstring", "Docstrings on functions and classes", Sometimes, rules::docstrings::function_docstring),
    MultilineModuleDocstring => ("SMP003", "multiline-module-docstring", "Module docstrings longer than one line", None, rules::docstrings::multiline_module_docstring),
    TestModuleDocstring => ("SMP004", "test-module-docstring", "Docstrings on test modules", Sometimes, rules::docstrings::test_module_docstring),
    LongParagraph => ("SMP005", "long-paragraph", "Comment or docstring paragraphs over 100 words", None, rules::comments::long_paragraph),
    SingleLetterName => ("SMP010", "single-letter-name", "Single-letter names outside loop and comprehension targets", None, rules::names::single_letter_name),
    DefinitionBeforeUse => ("SMP020", "definition-before-use", "Helpers defined above their first caller, or away from it", Sometimes, rules::order::definition_before_use),
    ForwardingFunction => ("SMP030", "forwarding-function", "Private functions that only forward to another call", None, rules::functions::forwarding_function),
    DependencyImportGuard => ("SMP040", "dependency-import-guard", "`try`/`except ImportError` around a required dependency", Sometimes, rules::imports::dependency_import_guard),
    SingleUserConstant => ("SMP050", "single-user-constant", "Module constants only one function uses", Sometimes, rules::constants::single_user_constant),
    SuppressionWithoutReason => ("SMP060", "suppression-without-reason", "`noqa`, `type: ignore` and `pragma` comments with no reason", None, rules::comments::suppression_without_reason),
    WalrusCandidate => ("SMP100", "walrus-candidate", "An assignment the next `if` test could absorb with `:=`", Always, rules::walrus::walrus_candidate),
    MissingFinal => ("SMP101", "missing-final", "Module constants not annotated `Final`", Sometimes, rules::constants::missing_final),
    ModuleVariableCase => ("SMP102", "module-variable-case", "Module variables not `UPPER_CASE`, or wrongly `_`-prefixed", Sometimes, rules::names::module_variable_case),
    UnexportedPublicName => ("SMP103", "unexported-public-name", "Public functions and classes missing from `__all__`", Sometimes, rules::names::unexported_public_name),
    AllNotLast => ("SMP104", "all-not-last", "`__all__` not at the end of the module", Sometimes, rules::exports::all_not_last),
    CallableEllipsis => ("SMP105", "callable-ellipsis", "`Callable[..., T]`", None, rules::typing::callable_ellipsis),
    ObjectAnnotation => ("SMP106", "object-annotation", "`object` annotations outside `TypeIs` guards and dunders", None, rules::typing::object_annotation),
    VarsCall => ("SMP107", "vars-call", "`vars()` calls", None, rules::typing::vars_call),
    FutureAnnotations => ("SMP108", "future-annotations", "`from __future__ import annotations` on Python 3.14+", Always, rules::imports::future_annotations),
    InitModuleContent => ("SMP109", "init-module-content", "`__init__.py` holding more than a docstring", None, rules::imports::init_module_content),
    AbsoluteSiblingImport => ("SMP110", "absolute-sibling-import", "Absolute imports from the same package", Always, rules::imports::absolute_sibling_import),
    DataclassOptions => ("SMP111", "dataclass-options", "Dataclasses without `frozen` and `slots`", Always, rules::dataclasses::dataclass_options),
    StringConcatenation => ("SMP112", "string-concatenation", "`+` between strings", Sometimes, rules::strings::string_concatenation),
    MissingFromAll => ("SMP113", "missing-from-all", "Names other modules import that `__all__` omits", Sometimes, rules::exports::missing_from_all),
    DataclassKwOnly => ("SMP114", "dataclass-kw-only", "Dataclasses without `kw_only`", Always, rules::dataclasses::dataclass_kw_only),
    TestClass => ("SMP200", "test-class", "Test classes", Sometimes, rules::pytest::test_class),
    ParametrizeWithoutId => ("SMP201", "parametrize-without-id", "`parametrize` rows without `pytest.param(..., id=...)`", None, rules::pytest::parametrize_without_id),
    ConditionalInTest => ("SMP202", "conditional-in-test", "`if`, `match` and conditional expressions in tests", None, rules::pytest::conditional_in_test),
    MockAttributeAssignment => ("SMP203", "mock-attribute-assignment", "Mock attributes assigned after construction", Sometimes, rules::mocks::mock_attribute_assignment),
    MockWithoutSpec => ("SMP204", "mock-without-spec", "Mocks and patches without a spec", None, rules::mocks::mock_without_spec),
    ConstantAssert => ("SMP205", "constant-assert", "Assertions on a truthy constant", Sometimes, rules::pytest::constant_assert),
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FixAvailability {
    Always,
    Sometimes,
    None,
}

#[derive(Clone, Copy, Default)]
pub struct RuleSet(u64);

impl RuleSet {
    pub fn all() -> Self {
        Self::from_rules(Rule::ALL.iter().copied())
    }

    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Self {
        Self(rules.into_iter().fold(0, |bits, rule| bits | 1 << rule as u64))
    }

    pub fn from_selectors<S: AsRef<str>>(selectors: &[S]) -> Result<Self> {
        selectors
            .iter()
            .try_fold(Self::default(), |set, selector| Ok(set.union(Self::from_selector(selector.as_ref())?)))
    }

    fn from_selector(selector: &str) -> Result<Self> {
        if selector == "ALL" {
            return Ok(Self::all());
        }
        let set = Self::from_rules(
            Rule::ALL.iter().copied().filter(|rule| rule.code().starts_with(selector) || rule.name() == selector),
        );
        if set.0 == 0 {
            bail!("unknown rule selector `{selector}`");
        }
        Ok(set)
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn subtract(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn iter(self) -> impl Iterator<Item = Rule> {
        Rule::ALL.iter().copied().filter(move |rule| self.0 & 1 << *rule as u64 != 0)
    }
}
