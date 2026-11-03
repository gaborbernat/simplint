use std::collections::HashSet;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::last_segment;
use crate::rules::rest::{FieldKind, Owner, targets};

pub fn rest_missing_raises(checker: &Checker) {
    for target in targets(checker).into_iter().filter(|target| target.public) {
        let Owner::Function { def, .. } = target.owner else {
            continue;
        };
        let mut raises = Raises::default();
        raises.visit_body(&def.body);
        let documented: HashSet<&str> = target
            .docstring
            .fields(FieldKind::Raises)
            .filter_map(|field| field.subject())
            .map(|subject| subject.rsplit('.').next().unwrap_or(subject))
            .collect();
        let mut reported = HashSet::new();
        for name in raises.raised {
            if documented.contains(name) || !reported.insert(name) {
                continue;
            }
            checker.report(
                Rule::RestMissingRaises,
                target.docstring.range(),
                format!("`{}` raises `{name}`, which its docstring does not document; add `:raises {name}:`", def.name),
            );
        }
    }
}

// The exceptions a function body raises by name and lets escape: not re-raised caught ones, not those a surrounding
// `try` of the same function catches, and not `NotImplementedError`, which marks a method to override.
#[derive(Default)]
struct Raises<'a> {
    raised: Vec<&'a str>,
    caught: Vec<Vec<&'a str>>,
    bound: HashSet<&'a str>,
}

impl<'a> Visitor<'a> for Raises<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                let caught = handlers
                    .iter()
                    .flat_map(|handler| {
                        let ast::ExceptHandler::ExceptHandler(handler) = handler;
                        match handler.type_.as_deref() {
                            None => vec!["BaseException"],
                            Some(Expr::Tuple(tuple)) => tuple.elts.iter().filter_map(last_segment).collect(),
                            Some(single) => last_segment(single).into_iter().collect(),
                        }
                    })
                    .collect();
                self.caught.push(caught);
                self.visit_body(body);
                self.caught.pop();
                for handler in handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    self.bound.extend(handler.name.as_ref().map(ast::Identifier::as_str));
                    self.visit_body(&handler.body);
                }
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) => {
                let raised = exc.as_call_expr().map_or(exc.as_ref(), |call| call.func.as_ref());
                let Some(name) = last_segment(raised) else {
                    return;
                };
                let caught = self
                    .caught
                    .iter()
                    .flatten()
                    .any(|caught| *caught == name || matches!(*caught, "Exception" | "BaseException"));
                if name.starts_with(char::is_uppercase)
                    && name != "NotImplementedError"
                    && !self.bound.contains(name)
                    && !caught
                {
                    self.raised.push(name);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}
