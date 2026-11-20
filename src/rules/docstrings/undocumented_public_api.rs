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

// Ruff applies its own visibility and export rules when deciding whether to require a docstring.
struct RuffPublic<'a> {
    module_public: bool,
    // An explicit empty export list differs from having no export list.
    exports: Option<FxHashSet<&'a str>>,
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
                Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) if is_name(target, "__all__") => {
                    Self::add_exports(&mut exports, value.as_deref());
                }
                _ => {}
            }
        }
        Self { module_public: Self::module_public(checker), exports, typing_aliases }
    }

    // Ruff treats a declaration without a value as an empty export list.
    fn add_exports(exports: &mut Option<FxHashSet<&'a str>>, value: Option<&'a Expr>) {
        let names = exports.get_or_insert_with(FxHashSet::default);
        names.extend(value.into_iter().flat_map(Self::listed_names));
    }

    // Mirror the literal export forms recognized by Ruff's pydocstyle checks.
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

    fn reported_by(&self, checker: &Checker, item: &ApiItem) -> Option<RuffRule> {
        if !self.module_public {
            return None;
        }
        // Ruff tests the enclosing top-level name against the module's exports.
        let top = top_level(&item.name);
        if top.starts_with('_') || self.exports.as_ref().is_some_and(|names| !names.contains(top)) {
            return None;
        }
        let rule = Self::ruff_code(item)?;
        // Directly named overloads and overrides have already been filtered. Ruff recognizes aliased imports too.
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
