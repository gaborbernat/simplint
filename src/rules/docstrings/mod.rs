mod function_docstring;
mod rest;
mod rest_attribute_kind;
mod rest_duplicate_param;
mod rest_field_syntax;
mod rest_inline_markup;
mod rest_missing_attribute;
mod rest_missing_param;
mod rest_missing_raises;
mod rest_param_order;
mod rest_private_field;
mod rest_restating_field;
mod rest_returns;
mod rest_type_field;
mod rest_unknown_attribute;
mod rest_unknown_param;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use rest_attribute_kind::rest_attribute_kind;
pub(crate) use rest_duplicate_param::rest_duplicate_param;
pub(crate) use rest_field_syntax::rest_field_syntax;
pub(crate) use rest_inline_markup::rest_inline_markup;
pub(crate) use rest_missing_attribute::rest_missing_attribute;
pub(crate) use rest_missing_param::rest_missing_param;
pub(crate) use rest_missing_raises::rest_missing_raises;
pub(crate) use rest_param_order::rest_param_order;
pub(crate) use rest_private_field::rest_private_field;
pub(crate) use rest_restating_field::rest_restating_field;
pub(crate) use rest_returns::rest_returns;
pub(crate) use rest_type_field::rest_type_field;
pub(crate) use rest_unknown_attribute::rest_unknown_attribute;
pub(crate) use rest_unknown_param::rest_unknown_param;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use std::path::Path;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::project::Exposure;
use crate::rules::exports::Exports;
use crate::rules::{last_segment, module_statements};

enum ApiKind {
    Function,
    Class,
    Method,
    Property,
    Variable,
}

struct ApiItem<'a> {
    stmt: &'a Stmt,
    name: String,
    range: TextRange,
    kind: ApiKind,
    // The file that re-exports the item to users, when its own module does not offer it.
    via: Option<&'a Path>,
}

// The public API of a library is what its users can import: what a public module offers, through `__all__` or, without
// one, every name it defines without a leading underscore, and what a public module re-exports from elsewhere, plus the
// public members of those classes. An application has none. When `__all__` is built dynamically the exports are
// unknown, so every name without a leading underscore might be public.
pub(crate) struct PublicApi<'a> {
    items: Vec<ApiItem<'a>>,
    maybe_public: Vec<TextRange>,
    // Members of a public class whose docstring is shown when present but need none: dunders such as `__init__`, which
    // autodoc can render, overrides, which inherit the base docstring, and property setters.
    optional: Vec<TextRange>,
}

impl<'a> PublicApi<'a> {
    pub(crate) fn of(checker: &Checker<'a>, body: &'a [Stmt]) -> Self {
        let project = checker.project();
        let role = project.role();
        let exports = checker.dunder_all();
        // The index answers for every file of the run; a module read from standard input has none, so its own offer is
        // read here too. A name another module imports counts, since `missing-from-all` lists it in `__all__`.
        let offered = |name: &str| {
            role.public
                && (!name.starts_with('_') && project.is_imported(name)
                    || match exports {
                        Exports::Listed(names) => names.contains(&name),
                        Exports::Absent => !name.starts_with('_'),
                        Exports::Dynamic => false,
                    })
        };
        let reach = |name: &str| {
            project.user_facing(name).or_else(|| offered(name).then_some(Exposure::Offered)).map(Exposure::reexporter)
        };
        let dynamic = matches!(exports, Exports::Dynamic);
        let mut api = Self { items: Vec::new(), maybe_public: Vec::new(), optional: Vec::new() };
        // Definitions in top-level `if` and `try` blocks, as a version switch or an optional accelerator, are the module's too.
        for stmt in module_statements(body) {
            match stmt {
                Stmt::FunctionDef(function) => {
                    let name = function.name.as_str();
                    match reach(name) {
                        // An `@overload` variant of a public function may carry a docstring, but needs none.
                        Some(_) if is_overload(&function.decorator_list) => api.optional.push(function.range),
                        Some(via) => {
                            let range = function.name.range;
                            let kind = ApiKind::Function;
                            api.items.push(ApiItem { stmt, name: name.to_owned(), range, kind, via });
                        }
                        None if dynamic && !name.starts_with('_') => api.maybe_public.push(function.range),
                        None => {}
                    }
                }
                Stmt::ClassDef(class) => match reach(class.name.as_str()) {
                    Some(via) => api.class(stmt, class, class.name.as_str(), via),
                    None if dynamic && !class.name.starts_with('_') => api.maybe_public.push(class.range),
                    None => {}
                },
                Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                    for target in targets.iter().filter_map(Expr::as_name_expr) {
                        if let Some(via) = reach(target.id.as_str()) {
                            api.variable(stmt, target, via);
                        }
                    }
                }
                Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => {
                    if let Some(target) = target.as_name_expr()
                        && let Some(via) = reach(target.id.as_str())
                    {
                        api.variable(stmt, target, via);
                    }
                }
                _ => {}
            }
        }
        api
    }

    fn variable(&mut self, stmt: &'a Stmt, target: &'a ast::ExprName, via: Option<&'a Path>) {
        let name = target.id.to_string();
        self.items.push(ApiItem { stmt, name, range: target.range, kind: ApiKind::Variable, via });
    }

    // A public class, its public methods and properties, and its public nested classes with theirs.
    fn class(&mut self, stmt: &'a Stmt, class: &'a ast::StmtClassDef, name: &str, via: Option<&'a Path>) {
        self.items.push(ApiItem { stmt, name: name.to_owned(), range: class.name.range, kind: ApiKind::Class, via });
        // Members in `if` and `try` blocks of the class body, as a version switch, are the class's too.
        for member in module_statements(&class.body) {
            match member {
                Stmt::ClassDef(nested) if !nested.name.starts_with('_') => {
                    self.class(member, nested, &format!("{name}.{}", nested.name), via);
                }
                Stmt::FunctionDef(method) => {
                    let method_name = method.name.as_str();
                    let decorators = decorator_names(&method.decorator_list);
                    if is_dunder(method_name)
                        || decorators
                            .iter()
                            .any(|decorator| matches!(*decorator, "override" | "setter" | "deleter" | "overload"))
                    {
                        self.optional.push(member.range());
                    } else if !method_name.starts_with('_') {
                        let kind = if decorators.contains(&"property") { ApiKind::Property } else { ApiKind::Method };
                        self.items.push(ApiItem {
                            stmt: member,
                            name: format!("{name}.{method_name}"),
                            range: method.name.range,
                            kind,
                            via,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Whether the definition at `range` is part of the public API, as opposed to optional or possibly public.
    pub(crate) fn is_public(&self, range: TextRange) -> bool {
        self.items.iter().any(|item| item.stmt.range() == range)
    }

    fn covers(&self, range: TextRange) -> bool {
        self.items.iter().any(|item| item.stmt.range() == range)
            || self.optional.contains(&range)
            || self.maybe_public.iter().any(|public| public.contains_range(range))
    }
}

fn is_overload(decorators: &[ast::Decorator]) -> bool {
    decorator_names(decorators).contains(&"overload")
}

// The last segment of each decorator, so `typing.override` and `@name.setter` read as `override` and `setter`.
fn decorator_names(decorators: &[ast::Decorator]) -> Vec<&str> {
    decorators.iter().filter_map(|decorator| last_segment(&decorator.expression)).collect()
}

// A variable is documented the two ways Sphinx autodoc reads: a string right after it, or `#:` comments right above.
pub(crate) fn is_documented_variable(checker: &Checker, body: &[Stmt], stmt: &Stmt) -> bool {
    let next = body.iter().skip_while(|candidate| candidate.range() != stmt.range()).nth(1);
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}
