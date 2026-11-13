use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Stmt};
use rustc_hash::FxHashSet;

use super::raised_class;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner};
use crate::rules::exception_classes::{canonical_name, handled_types, lineage_names};
use crate::syntax::last_segment;

pub fn rst_missing_raises(checker: &Checker) {
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public(checker)) {
        let Owner::Function { def, .. } = target.owner else {
            continue;
        };
        let mut raises = Raises { checker, raised: Vec::new(), caught: Vec::new(), bound: FxHashSet::default() };
        raises.visit_body(&def.body);
        let documented: FxHashSet<&str> = target.docstring.subjects(&[FieldKind::Raises]).map(canonical_name).collect();
        let mut missing: Vec<&str> = Vec::new();
        // A field for a base documents its subclasses too, as `:raises OSError:` does `FileNotFoundError`.
        for (name, lineage) in raises.raised {
            if !lineage.iter().any(|class| documented.contains(class)) && !missing.contains(&name) {
                missing.push(name);
            }
        }
        // The description of a raised exception is for a person to write, so the report carries no fix.
        for name in missing {
            checker.report(
                Rule::RstMissingRaises,
                target.docstring.range(),
                format!("`{}` raises `{name}`, which its docstring does not document; add `:raises {name}:`", def.name),
            );
        }
    }
}

// The exceptions a function body raises by name and lets escape, each with the classes it derives from: not re-raised
// caught ones, not those a surrounding `try` of the same function catches, and not `NotImplementedError`, which marks a
// method to override.
struct Raises<'c, 'a> {
    checker: &'c Checker<'a>,
    raised: Vec<(&'a str, Vec<&'a str>)>,
    caught: Vec<Vec<&'a str>>,
    bound: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for Raises<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                let caught = handlers.iter().flat_map(|handler| caught_names(self.checker, handler)).collect();
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
                let class = raised_class(exc);
                let (Some(name), Some(lineage)) = (last_segment(class), lineage_names(self.checker, class)) else {
                    return;
                };
                // A class whose bases are not known is taken to derive from `Exception`, as a project's own do.
                let unknown = !lineage.contains(&"BaseException");
                let caught = self.caught.iter().flatten().any(|caught| {
                    lineage.contains(caught) || unknown && matches!(*caught, "Exception" | "BaseException")
                });
                if name.starts_with(char::is_uppercase)
                    && name != "NotImplementedError"
                    && !self.bound.contains(name)
                    && !caught
                {
                    self.raised.push((name, lineage));
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

// The classes a handler catches, by the name a docstring gives them; a bare `except` catches `BaseException`.
fn caught_names<'a>(checker: &Checker<'a>, handler: &'a ast::ExceptHandler) -> Vec<&'a str> {
    let ast::ExceptHandler::ExceptHandler(handler) = handler;
    handled_types(handler).map_or_else(
        || vec!["BaseException"],
        |types| types.into_iter().filter_map(|expr| lineage_names(checker, expr).map(|lineage| lineage[0])).collect(),
    )
}
