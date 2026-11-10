use ruff_linter::rules::flake8_type_checking::settings::RuntimeSemantics;
use ruff_linter::settings::LinterSettings;
use ruff_python_ast::helpers::map_callable;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::{self as ast, Decorator};
use ruff_python_semantic::SemanticModel;
use ruff_python_semantic::analyze::{class, typing};
use rustc_hash::FxHashMap;

// The runtime semantics `flake8-type-checking` gives annotations through the base classes and decorators it lists, which
// decide whether Ruff's import-private-name exempts a read in an annotation.
pub struct Runtime<'a> {
    base_classes: &'a FxHashMap<String, RuntimeSemantics>,
    decorators: &'a FxHashMap<String, RuntimeSemantics>,
}

impl<'a> Runtime<'a> {
    pub fn new(settings: &'a LinterSettings) -> Self {
        let flake8 = &settings.flake8_type_checking;
        Self { base_classes: &flake8.runtime_evaluated_base_classes, decorators: &flake8.runtime_evaluated_decorators }
    }

    pub fn class(&self, class_def: &ast::StmtClassDef, semantic: &SemanticModel) -> RuntimeSemantics {
        let semantics = self.base_class(class_def, semantic);
        if semantics == RuntimeSemantics::Required {
            return semantics;
        }
        semantics.min(self.decorators(&class_def.decorator_list, semantic))
    }

    pub fn function(&self, function_def: &ast::StmtFunctionDef, semantic: &SemanticModel) -> RuntimeSemantics {
        self.decorators(&function_def.decorator_list, semantic)
    }

    fn base_class(&self, class_def: &ast::StmtClassDef, semantic: &SemanticModel) -> RuntimeSemantics {
        let mut result = RuntimeSemantics::Default;
        if self.base_classes.is_empty() {
            return result;
        }
        class::any_qualified_base_class(class_def, semantic, |qualified_name| {
            match self
                .base_classes
                .iter()
                .find(|(base_class, _)| QualifiedName::from_dotted_name(base_class) == qualified_name)
            {
                Some((_, RuntimeSemantics::Required)) => {
                    result = RuntimeSemantics::Required;
                    true
                }
                Some((_, semantics)) => {
                    result = result.min(*semantics);
                    false
                }
                None => false,
            }
        });
        result
    }

    fn decorators(&self, decorator_list: &[Decorator], semantic: &SemanticModel) -> RuntimeSemantics {
        let mut result = RuntimeSemantics::Default;
        if self.decorators.is_empty() {
            return result;
        }
        for decorator in decorator_list {
            let expression = map_callable(&decorator.expression);
            let listed = semantic
                .resolve_qualified_name(expression)
                .or_else(|| typing::resolve_assignment(expression, semantic))
                .and_then(|qualified_name| {
                    self.decorators
                        .iter()
                        .find(|(decorator, _)| QualifiedName::from_dotted_name(decorator) == qualified_name)
                });
            match listed {
                Some((_, RuntimeSemantics::Required)) => return RuntimeSemantics::Required,
                Some((_, semantics)) => result = result.min(*semantics),
                None => {}
            }
        }
        result
    }
}
