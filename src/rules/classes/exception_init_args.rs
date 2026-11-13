use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::exception_classes::{ClassKey, derives_from_base_exception, lineage};
use crate::syntax::REBUILD_HOOKS;

// `pickle` and `copy` rebuild an exception by calling its class with its `args`, which `BaseException.__init__` sets to
// the arguments it receives. A subclass whose `__init__` passes its parent fewer or more positional values than it
// takes itself cannot be rebuilt from them.
pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::ClassDef(class) = stmt else {
        return;
    };
    let checker = walker.checker;
    let lineages: Vec<Vec<ClassKey>> = class.bases().iter().filter_map(|base| lineage(checker, base)).collect();
    let is_exception = lineages.iter().any(|lineage| derives_from_base_exception(lineage))
        || class
            .bases()
            .iter()
            .any(|base| base.as_name_expr().is_some_and(|name| checker.project().is_exception_type(name.id.as_str())));
    if !is_exception {
        return;
    }
    // A base of this file that defines a rebuilding method passes it on.
    let rebuilt = std::iter::once(class)
        .chain(lineages.iter().flatten().filter_map(|key| match key {
            ClassKey::Local(range) => checker.class_defs().iter().copied().find(|class| class.range() == *range),
            ClassKey::Known(_) => None,
        }))
        .any(|class| {
            class
                .body
                .iter()
                .filter_map(Stmt::as_function_def_stmt)
                .any(|def| REBUILD_HOOKS.contains(&def.name.as_str()))
        });
    if rebuilt {
        return;
    }
    let Some(init) =
        class.body.iter().filter_map(Stmt::as_function_def_stmt).find(|def| def.name.as_str() == "__init__")
    else {
        return;
    };
    let parameters = &init.parameters;
    let positional: Vec<&ast::ParameterWithDefault> =
        parameters.posonlyargs.iter().chain(&parameters.args).skip(1).collect();
    let required = positional.iter().filter(|parameter| parameter.default.is_none()).count();
    let name = class.name.as_str();
    if let Some(keyword) = parameters.kwonlyargs.iter().find(|parameter| parameter.default.is_none()) {
        checker.report(
            Rule::ExceptionInitArgs,
            keyword.parameter.name.range(),
            format!(
                "`{name}` requires the keyword `{}`, which its `args` cannot carry, so `pickle` and `copy` cannot rebuild \
                 it; make it positional and pass it to the parent `__init__`, or define `__reduce__`",
                keyword.parameter.name
            ),
        );
        return;
    }
    let mut calls = ParentInits { bases: class.bases(), found: Vec::new() };
    calls.visit_body(&init.body);
    for call in calls.found {
        let ast::Arguments { args, keywords, .. } = &call.arguments;
        if args.iter().any(Expr::is_starred_expr) || keywords.iter().any(|keyword| keyword.arg.is_none()) {
            continue;
        }
        let passed = args.len() - usize::from(!is_super_call(&call.func));
        if passed >= required && (parameters.vararg.is_some() || passed <= positional.len()) {
            continue;
        }
        checker.report(
            Rule::ExceptionInitArgs,
            call.range(),
            format!(
                "`{name}.__init__` takes {} but passes {passed} to its parent, so `pickle` and `copy` rebuild it with \
                 {passed} and fail; pass the arguments on, or define `__reduce__`",
                takes(required, positional.len()),
            ),
        );
    }
}

fn takes(required: usize, total: usize) -> String {
    let count = |count: usize| format!("{count} positional argument{}", if count == 1 { "" } else { "s" });
    if required == total { count(total) } else { format!("{required} to {}", count(total)) }
}

// `super().__init__(...)` or `super(Class, self).__init__(...)`, whose arguments start after `self`, or
// `Base.__init__(self, ...)`.
fn is_super_call(func: &Expr) -> bool {
    func.as_attribute_expr().is_some_and(|attribute| {
        attribute
            .value
            .as_call_expr()
            .is_some_and(|call| call.func.as_name_expr().is_some_and(|name| name.id == "super"))
    })
}

// The calls in `__init__` to the parent's `__init__`, outside nested functions and classes.
struct ParentInits<'a> {
    bases: &'a [Expr],
    found: Vec<&'a ast::ExprCall>,
}

impl<'a> Visitor<'a> for ParentInits<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            visitor::walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr
            && let Expr::Attribute(attribute) = call.func.as_ref()
            && attribute.attr.as_str() == "__init__"
            && (is_super_call(&call.func) || self.names_base(&attribute.value))
        {
            self.found.push(call);
        }
        visitor::walk_expr(self, expr);
    }
}

impl ParentInits<'_> {
    fn names_base(&self, value: &Expr) -> bool {
        let text = |expr: &Expr| match expr {
            Expr::Name(name) => Some(name.id.to_string()),
            Expr::Attribute(attribute) => Some(attribute.attr.to_string()),
            _ => None,
        };
        let Some(value) = text(value) else {
            return false;
        };
        self.bases.iter().any(|base| text(base).is_some_and(|base| base == value))
            || ["Exception", "BaseException"].contains(&value.as_str())
    }
}
