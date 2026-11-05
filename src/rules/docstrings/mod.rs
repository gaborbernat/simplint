mod function_docstring;
mod long_docstring_paragraph;
pub(crate) mod rst;
mod rst_attribute_kind;
mod rst_duplicate_param;
mod rst_empty_field;
mod rst_field_syntax;
mod rst_inline_markup;
mod rst_missing_attribute;
mod rst_missing_param;
mod rst_missing_raises;
mod rst_param_order;
mod rst_private_field;
mod rst_restating_field;
mod rst_returns;
mod rst_type_field;
mod rst_unknown_attribute;
mod rst_unknown_param;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use long_docstring_paragraph::long_docstring_paragraph;
pub(crate) use rst_attribute_kind::rst_attribute_kind;
pub(crate) use rst_duplicate_param::rst_duplicate_param;
pub(crate) use rst_empty_field::rst_empty_field;
pub(crate) use rst_field_syntax::rst_field_syntax;
pub(crate) use rst_inline_markup::rst_inline_markup;
pub(crate) use rst_missing_attribute::rst_missing_attribute;
pub(crate) use rst_missing_param::rst_missing_param;
pub(crate) use rst_missing_raises::rst_missing_raises;
pub(crate) use rst_param_order::rst_param_order;
pub(crate) use rst_private_field::rst_private_field;
pub(crate) use rst_restating_field::rst_restating_field;
pub(crate) use rst_returns::rst_returns;
pub(crate) use rst_type_field::rst_type_field;
pub(crate) use rst_unknown_attribute::rst_unknown_attribute;
pub(crate) use rst_unknown_param::rst_unknown_param;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use std::path::Path;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::project::Exposure;
use crate::syntax::{Exports, Visibility, decorator_names, has_decorator, module_statements};

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
    // The statement after a variable's in its block, where its docstring goes.
    next: Option<&'a Stmt>,
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
    // Built once per file; rules read it through `Checker::public_api`.
    pub(crate) fn of(checker: &Checker<'a>) -> Self {
        let body = checker.body();
        let project = checker.project();
        let role = project.role();
        let exports = checker.dunder_all();
        // The index answers for every file of the run; a module read from standard input has none, so its own offer is
        // read here too. A name another module imports counts, since `missing-from-all` lists it in `__all__`.
        let offered = |name: &str| {
            role.public
                && (Visibility::of(name) == Visibility::Public && project.is_imported(name)
                    || match exports {
                        Exports::Listed(names) => names.contains(&name),
                        Exports::Absent => Visibility::of(name) == Visibility::Public,
                        Exports::Dynamic => false,
                    })
        };
        let reach = |name: &str| {
            project.user_facing(name).or_else(|| offered(name).then_some(Exposure::Offered)).map(Exposure::reexporter)
        };
        let dynamic = matches!(exports, Exports::Dynamic);
        let mut api = Self { items: Vec::new(), maybe_public: Vec::new(), optional: Vec::new() };
        // Definitions in top-level `if`, `try` and `match` blocks, as a version switch or an optional accelerator, are
        // the module's too.
        for (stmt, next) in module_statements_with_next(body) {
            match stmt {
                Stmt::FunctionDef(function) => {
                    let name = function.name.as_str();
                    match reach(name) {
                        // An `@overload` variant of a public function may carry a docstring, but needs none.
                        Some(_) if has_decorator(&function.decorator_list, &["overload"]) => {
                            api.optional.push(function.range);
                        }
                        Some(via) => {
                            let range = function.name.range;
                            let kind = ApiKind::Function;
                            api.items.push(ApiItem { stmt, name: name.to_owned(), range, kind, next: None, via });
                        }
                        None if dynamic && Visibility::of(name) == Visibility::Public => {
                            api.maybe_public.push(function.range);
                        }
                        None => {}
                    }
                }
                Stmt::ClassDef(class) => match reach(class.name.as_str()) {
                    Some(via) => api.class(stmt, class, class.name.as_str(), via),
                    None if dynamic && Visibility::of(&class.name) == Visibility::Public => {
                        api.maybe_public.push(class.range);
                    }
                    None => {}
                },
                Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                    for target in targets.iter().filter_map(Expr::as_name_expr) {
                        if let Some(via) = reach(target.id.as_str()) {
                            api.variable(stmt, next, target, via);
                        }
                    }
                }
                Stmt::AnnAssign(ast::StmtAnnAssign { target: name, .. })
                | Stmt::TypeAlias(ast::StmtTypeAlias { name, .. }) => {
                    if let Some(target) = name.as_name_expr()
                        && let Some(via) = reach(target.id.as_str())
                    {
                        api.variable(stmt, next, target, via);
                    }
                }
                _ => {}
            }
        }
        api
    }

    fn variable(&mut self, stmt: &'a Stmt, next: Option<&'a Stmt>, target: &'a ast::ExprName, via: Option<&'a Path>) {
        let name = target.id.to_string();
        self.items.push(ApiItem { stmt, name, range: target.range, kind: ApiKind::Variable, next, via });
    }

    // A public class, its public methods and properties, and its public nested classes with theirs.
    fn class(&mut self, stmt: &'a Stmt, class: &'a ast::StmtClassDef, name: &str, via: Option<&'a Path>) {
        let range = class.name.range;
        self.items.push(ApiItem { stmt, name: name.to_owned(), range, kind: ApiKind::Class, next: None, via });
        // Members in `if` and `try` blocks of the class body, as a version switch, are the class's too.
        for member in module_statements(&class.body) {
            match member {
                Stmt::ClassDef(nested) if Visibility::of(&nested.name) == Visibility::Public => {
                    self.class(member, nested, &format!("{name}.{}", nested.name), via);
                }
                Stmt::FunctionDef(method) => {
                    let method_name = method.name.as_str();
                    let decorators = decorator_names(&method.decorator_list);
                    let visibility = Visibility::of(method_name);
                    if visibility == Visibility::Dunder
                        || decorators
                            .iter()
                            .any(|decorator| matches!(*decorator, "override" | "setter" | "deleter" | "overload"))
                    {
                        self.optional.push(member.range());
                    } else if visibility == Visibility::Public {
                        let kind = if decorators.contains(&"property") { ApiKind::Property } else { ApiKind::Method };
                        self.items.push(ApiItem {
                            stmt: member,
                            name: format!("{name}.{method_name}"),
                            range: method.name.range,
                            kind,
                            next: None,
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

    // Whether the definition at `range` may be part of the public API, which includes optional and possibly public ones.
    pub(crate) fn covers(&self, range: TextRange) -> bool {
        self.items.iter().any(|item| item.stmt.range() == range)
            || self.optional.contains(&range)
            || self.maybe_public.iter().any(|public| public.contains_range(range))
    }
}

// The statements `module_statements` yields, each with the one after it in its own block, which may be inside an `if`,
// `try` or `match`.
fn module_statements_with_next(body: &[Stmt]) -> Vec<(&Stmt, Option<&Stmt>)> {
    fn collect<'a>(body: &'a [Stmt], found: &mut Vec<(&'a Stmt, Option<&'a Stmt>)>) {
        for (index, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                    collect(body, found);
                    for clause in elif_else_clauses {
                        collect(&clause.body, found);
                    }
                }
                Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                    collect(body, found);
                    for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                        collect(&handler.body, found);
                    }
                    collect(orelse, found);
                    collect(finalbody, found);
                }
                Stmt::Match(ast::StmtMatch { cases, .. }) => {
                    for case in cases {
                        collect(&case.body, found);
                    }
                }
                _ => found.push((stmt, body.get(index + 1))),
            }
        }
    }
    let mut found = Vec::new();
    collect(body, &mut found);
    found
}

// A variable is documented the two ways Sphinx autodoc reads: a string right after it, or `#:` comments right above.
fn is_documented_variable(checker: &Checker, stmt: &Stmt, next: Option<&Stmt>) -> bool {
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}
