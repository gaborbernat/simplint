use ruff_linter::fs::relativize_path;
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::{is_documented_variable, ruff_ignores_function};
use crate::rules::public_api::{ApiItem, ApiKind};
use crate::syntax::{alias_binding, docstring, is_name, scope_statements, top_level};

pub fn undocumented_public_api(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let api = checker.state().public_api(checker);
    let ruff = RuffPublic::of(checker);
    for item in api.items.iter().filter(|item| item.docs) {
        let documented = match item.stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) | Stmt::ClassDef(ast::StmtClassDef { body, .. }) => {
                docstring(body).is_some()
            }
            _ => is_documented_variable(checker, item.stmt, item.next),
        };
        // Ruff's D100 to D107 report these definitions when the project selects them, so this rule leaves them to Ruff.
        let left_to_ruff =
            ruff.reported_by(checker, item).is_some_and(|rule| checker.settings.from_ruff.ruff_runs(rule));
        if documented || left_to_ruff {
            continue;
        }
        let name = &item.name;
        let reason = item.via.map_or_else(String::new, |via| format!("; `{}` re-exports it", relativize_path(via)));
        let message = match item.kind {
            ApiKind::Variable => format!(
                "Public variable `{name}` is undocumented; add a docstring below it or a `#:` comment above it{reason}"
            ),
            ApiKind::Function => {
                format!("Public function `{name}` has no docstring; document it for its users{reason}")
            }
            ApiKind::Class => format!("Public class `{name}` has no docstring; document it for its users{reason}"),
            ApiKind::Method => format!("Public method `{name}` has no docstring; document it for its users{reason}"),
            ApiKind::Property => {
                format!("Public property `{name}` has no docstring; document it for its users{reason}")
            }
        };
        checker.report(Rule::UndocumentedPublicApi, item.range, message);
    }
}

// The definitions Ruff's pydocstyle rules D100 to D107 check, read as Ruff reads them. A module is public unless a part
// of its module path starts with `_`, or, outside a package, its file name does. A top-level function or class is
// public when its name does not start with `_` and the module's `__all__` bindings, taken together, list it, where the
// module binds one; a member is public when its enclosing class is.
struct RuffPublic<'a> {
    module_public: bool,
    // Names listed by the module's `__all__` bindings together, or `None` when it has none.
    exports: Option<FxHashSet<&'a str>>,
    // Aliases of `overload` and `override` from `typing`, which Ruff resolves by their original name.
    typing_aliases: FxHashMap<&'a str, &'a str>,
}

impl<'a> RuffPublic<'a> {
    fn of(checker: &Checker<'a>) -> Self {
        let mut exports: Option<FxHashSet<&'a str>> = None;
        let mut typing_aliases = FxHashMap::default();
        for stmt in scope_statements(checker.body()) {
            match stmt {
                Stmt::ImportFrom(ast::StmtImportFrom { module, names, level: 0, .. })
                    if module
                        .as_ref()
                        .is_some_and(|module| matches!(module.as_str(), "typing" | "typing_extensions")) =>
                {
                    for alias in names.iter().filter(|alias| matches!(alias.name.as_str(), "overload" | "override")) {
                        typing_aliases.insert(alias_binding(alias), alias.name.as_str());
                    }
                }
                Stmt::Assign(ast::StmtAssign { targets, value, .. })
                    if targets.first().is_some_and(|target| is_name(target, "__all__")) =>
                {
                    Self::add_exports(&mut exports, Some(value.as_ref()));
                }
                Stmt::AugAssign(ast::StmtAugAssign { target, value, .. }) if is_name(target, "__all__") => {
                    Self::add_exports(&mut exports, Some(value.as_ref()));
                }
                // Ruff reads a binding without a value too, as `__all__: list[str]` has, and it lists nothing.
                Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) if is_name(target, "__all__") => {
                    Self::add_exports(&mut exports, value.as_deref());
                }
                _ => {}
            }
        }
        Self { module_public: Self::module_public(checker), exports, typing_aliases }
    }

    // A binding without a value still makes the exports present, and lists nothing.
    fn add_exports(exports: &mut Option<FxHashSet<&'a str>>, value: Option<&'a Expr>) {
        let names = exports.get_or_insert_with(FxHashSet::default);
        names.extend(value.into_iter().flat_map(Self::listed_names));
    }

    // Names as Ruff reads them: string elements of lists and tuples joined with `+`, or passed to `list()` or
    // `tuple()`. Any other value lists none.
    fn listed_names(value: &Expr) -> Vec<&str> {
        match value {
            Expr::BinOp(ast::ExprBinOp { left, right, .. }) => {
                let mut names = Self::listed_names(left);
                names.extend(Self::elements(right).into_iter().flatten().filter_map(Self::string_value));
                names
            }
            _ => Self::elements(value).into_iter().flatten().filter_map(Self::string_value).collect(),
        }
    }

    fn elements(value: &Expr) -> Option<&[Expr]> {
        match value {
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => Some(elts),
            Expr::Named(ast::ExprNamed { value, .. }) => Self::elements(value),
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if arguments.keywords.is_empty() && Self::is_sequence_builtin(func) =>
            {
                match &*arguments.args {
                    [
                        Expr::List(ast::ExprList { elts, .. })
                        | Expr::Set(ast::ExprSet { elts, .. })
                        | Expr::Tuple(ast::ExprTuple { elts, .. }),
                    ] => Some(elts),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn is_sequence_builtin(func: &Expr) -> bool {
        map_subscript(func).as_name_expr().is_some_and(|name| matches!(name.id.as_str(), "list" | "tuple"))
    }

    fn string_value(expr: &Expr) -> Option<&str> {
        expr.as_string_literal_expr().map(|literal| literal.value.to_str())
    }

    // Outside a package, the file name stands in for the module path.
    fn module_public(checker: &Checker) -> bool {
        if let Some(path) = checker.module_path() {
            return !path.iter().any(|part| Self::is_private_part(part));
        }
        let name = checker.file_name();
        let stem = name.rfind('.').map_or(name, |dot| &name[..dot]);
        !Self::is_private_part(stem)
    }

    fn is_private_part(part: &str) -> bool {
        part.starts_with('_') && !(part.starts_with("__") && part.ends_with("__"))
    }

    // The Ruff rule that reports the item as undocumented, when Ruff considers it public and does not exempt it as an
    // overload or an override. Every undocumented item this rule reads is one of those, or a variable Ruff never
    // reports.
    fn reported_by(&self, checker: &Checker, item: &ApiItem) -> Option<RuffRule> {
        if !self.module_public {
            return None;
        }
        // A dotted name is a member, whose visibility follows the class it sits in, so only the outermost name matters.
        let top = top_level(&item.name);
        if top.starts_with('_') || self.exports.as_ref().is_some_and(|names| !names.contains(top)) {
            return None;
        }
        let rule = Self::ruff_code(item)?;
        // Ruff exempts `@overload` on functions and `@overload` or `@override` on methods, which simplint already
        // exempts by name. Only a decorator imported under another name needs the alias map.
        // Classes have no such exemption.
        let Stmt::FunctionDef(function) = item.stmt else {
            return Some(rule);
        };
        if ruff_ignores_function(checker, &function.decorator_list) {
            return None;
        }
        let exempt: &[&str] = match item.kind {
            ApiKind::Function => &["overload"],
            _ => &["overload", "override"],
        };
        let exempted = function.decorator_list.iter().any(|decorator| {
            decorator
                .expression
                .as_name_expr()
                .and_then(|name| self.typing_aliases.get(name.id.as_str()))
                .is_some_and(|original| exempt.contains(original))
        });
        (!exempted).then_some(rule)
    }

    // Magic methods are not this rule's items, since D105 and D107 report them.
    fn ruff_code(item: &ApiItem) -> Option<RuffRule> {
        Some(match item.kind {
            ApiKind::Variable => return None,
            ApiKind::Function => RuffRule::UndocumentedPublicFunction,
            ApiKind::Class if item.name.contains('.') => RuffRule::UndocumentedPublicNestedClass,
            ApiKind::Class => RuffRule::UndocumentedPublicClass,
            ApiKind::Method | ApiKind::Property => RuffRule::UndocumentedPublicMethod,
        })
    }
}
