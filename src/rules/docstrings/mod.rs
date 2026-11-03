mod function_docstring;
mod multiline_module_docstring;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use multiline_module_docstring::multiline_module_docstring;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use std::path::Path;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::project::Exposure;
use crate::rules::exports::Exports;
use crate::rules::last_segment;

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
        for stmt in body {
            match stmt {
                Stmt::FunctionDef(function) => {
                    let name = function.name.as_str();
                    match reach(name) {
                        Some(via) => {
                            if !is_overload(&function.decorator_list) {
                                let range = function.name.range;
                                let kind = ApiKind::Function;
                                api.items.push(ApiItem { stmt, name: name.to_owned(), range, kind, via });
                            }
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
        for member in &class.body {
            match member {
                Stmt::ClassDef(nested) if !nested.name.starts_with('_') => {
                    self.class(member, nested, &format!("{name}.{}", nested.name), via);
                }
                Stmt::FunctionDef(method) => {
                    let method_name = method.name.as_str();
                    let decorators = decorator_names(&method.decorator_list);
                    if is_dunder(method_name)
                        || decorators.iter().any(|decorator| matches!(*decorator, "override" | "setter" | "deleter"))
                    {
                        self.optional.push(member.range());
                    } else if !method_name.starts_with('_') && !decorators.contains(&"overload") {
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
