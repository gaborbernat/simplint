mod function_docstring;
mod multiline_module_docstring;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use multiline_module_docstring::multiline_module_docstring;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
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
    name: &'a str,
    range: TextRange,
    kind: ApiKind,
}

// The public API is what `__all__` exports or another module imports, plus the public methods of an exported class.
// When `__all__` is built dynamically the exports are unknown, so every name without a leading underscore might be
// public.
struct PublicApi<'a> {
    items: Vec<ApiItem<'a>>,
    maybe_public: Vec<TextRange>,
    // Members of an exported class whose docstring is shown when present but need none: dunders such as `__init__`,
    // which autodoc can render, overrides, which inherit the base docstring, and property setters.
    optional: Vec<TextRange>,
}

impl<'a> PublicApi<'a> {
    fn of(checker: &Checker, body: &'a [Stmt]) -> Self {
        let exported = match checker.dunder_all() {
            Exports::Listed(names) => Some(names.as_slice()),
            Exports::Absent => Some(&[][..]),
            Exports::Dynamic => None,
        };
        // A name another module imports is public even before `__all__` lists it (SMP113 adds it), so its docstring
        // must neither be stripped as internal nor be missing.
        let is_public = |name: &str| {
            exported.as_ref().is_some_and(|names| names.contains(&name))
                || (!name.starts_with('_') && checker.project().is_imported(name))
        };
        let mut items = Vec::new();
        let mut maybe_public = Vec::new();
        let mut optional = Vec::new();
        for stmt in body {
            match stmt {
                Stmt::FunctionDef(function) => {
                    let name = function.name.as_str();
                    if is_public(name) && !is_overload(&function.decorator_list) {
                        items.push(ApiItem { stmt, name, range: function.name.range, kind: ApiKind::Function });
                    } else if exported.is_none() && !name.starts_with('_') {
                        maybe_public.push(function.range);
                    }
                }
                Stmt::ClassDef(class) => {
                    let name = class.name.as_str();
                    let public = is_public(name);
                    if public {
                        items.push(ApiItem { stmt, name, range: class.name.range, kind: ApiKind::Class });
                    } else if exported.is_none() && !name.starts_with('_') {
                        maybe_public.push(class.range);
                        continue;
                    } else {
                        continue;
                    }
                    for member in &class.body {
                        let Stmt::FunctionDef(method) = member else {
                            continue;
                        };
                        let name = method.name.as_str();
                        let decorators = decorator_names(&method.decorator_list);
                        if is_dunder(name)
                            || decorators
                                .iter()
                                .any(|decorator| matches!(*decorator, "override" | "setter" | "deleter"))
                        {
                            optional.push(member.range());
                        } else if !name.starts_with('_') && !decorators.contains(&"overload") {
                            let kind =
                                if decorators.contains(&"property") { ApiKind::Property } else { ApiKind::Method };
                            items.push(ApiItem { stmt: member, name, range: method.name.range, kind });
                        }
                    }
                }
                Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                    for target in targets.iter().filter_map(Expr::as_name_expr) {
                        if is_public(target.id.as_str()) {
                            items.push(ApiItem {
                                stmt,
                                name: target.id.as_str(),
                                range: target.range,
                                kind: ApiKind::Variable,
                            });
                        }
                    }
                }
                Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => {
                    if let Some(target) = target.as_name_expr().filter(|target| is_public(target.id.as_str())) {
                        items.push(ApiItem {
                            stmt,
                            name: target.id.as_str(),
                            range: target.range,
                            kind: ApiKind::Variable,
                        });
                    }
                }
                _ => {}
            }
        }
        Self { items, maybe_public, optional }
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
