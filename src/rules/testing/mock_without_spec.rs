use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::testing::recognize::{is_mock_constructor_except_autospec, is_mock_patch, is_patch_object};
use crate::semantic::Semantic;
use crate::syntax::{instance_writes, last_segment};

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    let checker = walker.checker;
    let Some(kind) = MockCall::of(checker.semantic(), &call.func) else {
        return;
    };
    let arguments = &call.arguments;
    let replaced = kind.replaces(arguments)
        || arguments.keywords.iter().any(|keyword| match keyword.arg.as_ref() {
            None => true,
            Some(arg) => {
                arg == "new" || (matches!(arg.as_str(), "autospec" | "new_callable") && is_set(&keyword.value))
            }
        });
    if replaced {
        return;
    }
    let positional = match kind {
        MockCall::Mock => arguments.args.first().filter(|value| is_set(value)).map(|value| ("spec", value)),
        MockCall::Patch | MockCall::PatchObject => None,
    };
    let spec = positional.or_else(|| {
        ["spec", "spec_set"].into_iter().find_map(|name| {
            arguments.find_keyword(name).filter(|keyword| is_set(&keyword.value)).map(|keyword| (name, &keyword.value))
        })
    });
    let wraps = arguments.find_keyword("wraps").is_some_and(|keyword| is_set(&keyword.value));
    let advice = match kind {
        MockCall::Mock => "build it with `create_autospec`",
        MockCall::Patch | MockCall::PatchObject => "pass `autospec=True`",
    };
    let message = match (spec, wraps) {
        (None, false) => format!("{} without a spec; {advice}", kind.noun()),
        (Some((name, value)), _) if !reads_unspecced_member(checker, call, value) => {
            format!("`{name}` does not check call signatures; {advice}")
        }
        (_, true) => format!("`wraps` does not check call signatures; {advice}"),
        (Some(_), false) => return,
    };
    let fix = match kind {
        MockCall::Mock => None,
        MockCall::Patch | MockCall::PatchObject => arguments
            .find_keyword("spec")
            .filter(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| literal.value))
            .map(|keyword| Fix::unsafe_edit(Edit::range_replacement("autospec=True".to_owned(), keyword.range()))),
    };
    checker.report_fix(
        Rule::MockWithoutSpec,
        call.range(),
        message,
        fix,
        "Pass `autospec=True` in place of `spec=True`",
    );
}

enum MockCall {
    Mock,
    Patch,
    PatchObject,
}

impl MockCall {
    fn of(semantic: &Semantic, func: &Expr) -> Option<Self> {
        match last_segment(func)? {
            _ if is_mock_constructor_except_autospec(func) => Some(Self::Mock),
            _ if is_mock_patch(semantic, func) => Some(Self::Patch),
            _ if is_patch_object(semantic, func) => Some(Self::PatchObject),
            _ => None,
        }
    }

    // `patch` replaces its target with its second positional argument, `patch.object` with its third, so no mock is made.
    fn replaces(&self, arguments: &ast::Arguments) -> bool {
        match self {
            Self::Mock => false,
            Self::Patch => arguments.args.len() >= 2,
            Self::PatchObject => arguments.args.len() >= 3,
        }
    }

    fn noun(&self) -> &'static str {
        match self {
            Self::Mock => "Mock",
            Self::Patch | Self::PatchObject => "Patch",
        }
    }
}

// Spelled out as `None` or `False`, a value is the default, which is no spec.
fn is_set(value: &Expr) -> bool {
    !matches!(value, Expr::NoneLiteral(_)) && value.as_boolean_literal_expr().is_none_or(|literal| literal.value)
}

// `create_autospec` cannot see instance attributes a constructor sets, and it evaluates properties. A spec for a class
// the module defines is accepted when the test reads one of those members through the mock: the variable the call is
// bound to, or that variable's `return_value`.
fn reads_unspecced_member(checker: &Checker, call: &ast::ExprCall, spec: &Expr) -> bool {
    let Expr::Name(name) = spec else {
        return false;
    };
    let Some(class) = checker.module_classes().get(name.id.as_str()) else {
        return false;
    };
    let Some(test) = enclosing_function(checker, call) else {
        return false;
    };
    let Some(mock) = bound_name(&test.body, call.range()) else {
        return false;
    };
    let members = unspecced_members(checker.semantic(), class);
    let mut reads = MemberRead { mock, members: &members, found: false };
    reads.visit_body(&test.body);
    reads.found
}

fn enclosing_function<'a>(checker: &'a Checker, call: &ast::ExprCall) -> Option<&'a ast::StmtFunctionDef> {
    checker
        .function_defs()
        .iter()
        .copied()
        .filter(|function| function.range().contains_range(call.range()))
        .min_by_key(|function| function.range().len())
}

fn bound_name(body: &[Stmt], call: TextRange) -> Option<&str> {
    let mut binding = BoundName { call, name: None };
    binding.visit_body(body);
    binding.name
}

// The variable a statement binds the call to: an assignment's target, or the `as` name of a `with` item.
struct BoundName<'a> {
    call: TextRange,
    name: Option<&'a str>,
}

impl<'a> Visitor<'a> for BoundName<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if value.range() == self.call => {
                self.name = targets.iter().find_map(|target| target.as_name_expr()).map(|name| name.id.as_str());
            }
            Stmt::With(ast::StmtWith { items, .. }) => {
                let bound = items
                    .iter()
                    .find(|item| item.context_expr.range() == self.call)
                    .and_then(|item| item.optional_vars.as_deref())
                    .and_then(Expr::as_name_expr);
                if let Some(name) = bound {
                    self.name = Some(name.id.as_str());
                }
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

fn unspecced_members<'a>(semantic: &Semantic, class: &'a ast::StmtClassDef) -> FxHashSet<&'a str> {
    let mut members: FxHashSet<&str> = instance_writes(class)
        .into_iter()
        .filter(|write| write.method.name.as_str() == "__init__")
        .map(|write| write.attr)
        .collect();
    members.extend(
        class
            .body
            .iter()
            .filter_map(Stmt::as_function_def_stmt)
            .filter(|method| method.decorator_list.iter().any(|decorator| is_property(semantic, decorator)))
            .map(|method| method.name.as_str()),
    );
    members
}

fn is_property(semantic: &Semantic, decorator: &ast::Decorator) -> bool {
    semantic.is_builtin(&decorator.expression, "property")
        || semantic.resolves_to(&decorator.expression, &["functools", "cached_property"])
}

struct MemberRead<'a, 'm> {
    mock: &'a str,
    members: &'m FxHashSet<&'a str>,
    found: bool,
}

impl<'a> Visitor<'a> for MemberRead<'a, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Load, .. })
                if self.members.contains(attr.as_str()) && self.is_mock(value) =>
            {
                self.found = true;
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

impl MemberRead<'_, '_> {
    fn is_mock(&self, value: &Expr) -> bool {
        match value {
            Expr::Name(name) => name.id.as_str() == self.mock,
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                attr.as_str() == "return_value" && self.is_mock(value)
            }
            _ => false,
        }
    }
}
