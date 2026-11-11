use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{for_each_body, is_name};

pub fn absolute_sibling_import(checker: &Checker) {
    let Some((_, package)) = checker.module_path().and_then(<[String]>::split_last) else {
        return;
    };
    // Ruff's `namespace-packages` makes a directory without `__init__.py` a package, yet pytest imports a test module
    // there as a top-level module, where a relative import fails.
    if !checker.has_sibling_init() {
        return;
    }
    // A module that expects some other `__name__` than its dotted path runs outside its package: as a script under
    // `"__main__"`, or as source sent to another interpreter, like execnet's `"__channelexec__"`.
    if checker.body().iter().any(compares_module_name) {
        return;
    }
    let current = package.join(".");
    let shipped = shipped_functions(checker.body());
    for stmt in checker.body() {
        // The body of a function sent as source runs on its own, with no package around it.
        if let Stmt::FunctionDef(function) = stmt
            && shipped.contains(function.name.as_str())
        {
            continue;
        }
        for_each_body(std::slice::from_ref(stmt), &mut |statements, _| {
            for stmt in statements {
                if let Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) = stmt {
                    let name = module.as_str();
                    let relative = if name == current {
                        Some(".".to_owned())
                    } else {
                        name.strip_prefix(&current)
                            .and_then(|rest| rest.strip_prefix('.'))
                            .map(|rest| format!(".{rest}"))
                    };
                    if let Some(relative) = relative {
                        let title = format!("Import from `{relative}`");
                        checker.report_fix(
                            Rule::AbsoluteSiblingImport,
                            module.range(),
                            format!("Import from the same package relatively: `from {relative} import ...`"),
                            Some(Fix::safe_edit(Edit::range_replacement(relative, module.range()))),
                            title,
                        );
                    }
                }
            }
        });
    }
}

fn compares_module_name(stmt: &Stmt) -> bool {
    let Stmt::If(ast::StmtIf { test, elif_else_clauses, .. }) = stmt else {
        return false;
    };
    std::iter::once(test.as_ref()).chain(elif_else_clauses.iter().filter_map(|clause| clause.test.as_ref())).any(
        |test| {
            let Expr::Compare(ast::ExprCompare { ops, operands, .. }) = test else {
                return false;
            };
            match (&**ops, &**operands) {
                ([ast::CmpOp::Eq | ast::CmpOp::NotEq], [left, right]) => {
                    (is_name(left, "__name__") && right.is_string_literal_expr())
                        || (left.is_string_literal_expr() && is_name(right, "__name__"))
                }
                _ => false,
            }
        },
    )
}

// `remote_exec` and `inspect.getsource` turn a function into source text that runs elsewhere.
fn shipped_functions(body: &[Stmt]) -> FxHashSet<&str> {
    let mut finder = ShippedFinder { names: FxHashSet::default() };
    finder.visit_body(body);
    finder.names
}

struct ShippedFinder<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for ShippedFinder<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr
            && let Some(Expr::Name(argument)) = arguments.args.first()
            && let Some(callee) = match func.as_ref() {
                Expr::Name(name) => Some(name.id.as_str()),
                Expr::Attribute(attribute) => Some(attribute.attr.as_str()),
                _ => None,
            }
            && matches!(callee, "remote_exec" | "getsource")
        {
            self.names.insert(argument.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}
