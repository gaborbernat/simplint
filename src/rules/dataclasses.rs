use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::Ranged;

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::class_defs;

pub fn dataclass_options(checker: &mut Checker) {
    require_options(checker, Rule::DataclassOptions, &["frozen", "slots"], |checker, class, explicit| {
        if explicit || !class.bases().is_empty() || checker.project().is_base_class(class.name.as_str()) {
            return Applicability::Unsafe;
        }
        let mut hazards = SlotHazards { found: false };
        hazards.visit_body(&class.body);
        let mutated = class.body.iter().any(|stmt| {
            stmt.as_ann_assign_stmt()
                .and_then(|field| field.target.as_name_expr())
                .is_some_and(|field| checker.project().is_attribute_assigned(field.id.as_str()))
        });
        if hazards.found || mutated { Applicability::Unsafe } else { Applicability::Safe }
    });
}

pub fn dataclass_kw_only(checker: &mut Checker) {
    require_options(checker, Rule::DataclassKwOnly, &["kw_only"], |_, _, _| Applicability::Unsafe);
}

fn require_options(
    checker: &mut Checker,
    rule: Rule,
    options: &[&str],
    applicability: impl Fn(&Checker, &ast::StmtClassDef, bool) -> Applicability,
) {
    let mut found = Vec::new();
    for class in class_defs(checker.body()) {
        for decorator in &class.decorator_list {
            let (func, arguments) = match &decorator.expression {
                Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), Some(arguments)),
                other => (other, None),
            };
            if !UnqualifiedName::from_expr(func)
                .is_some_and(|name| matches!(name.segments(), ["dataclass"] | ["dataclasses", "dataclass"]))
            {
                continue;
            }
            let keywords = arguments.map_or(&[][..], |arguments| &arguments.keywords);
            let missing: Vec<&str> = options
                .iter()
                .copied()
                .filter(|option| {
                    keywords
                        .iter()
                        .find(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == *option))
                        .is_none_or(|keyword| {
                            !keyword.value.as_boolean_literal_expr().is_some_and(|literal| literal.value)
                        })
                })
                .collect();
            if !missing.is_empty() {
                let explicit = keywords
                    .iter()
                    .any(|keyword| keyword.arg.as_ref().is_some_and(|arg| missing.contains(&arg.as_str())));
                found.push((class, decorator.expression.range(), func.range(), keywords, missing, explicit));
            }
        }
    }
    let wanted: Vec<String> = options.iter().map(|option| format!("{option}=True")).collect();
    for (class, range, func, keywords, missing, explicit) in found {
        let arguments: Vec<&str> = keywords
            .iter()
            .filter(|keyword| keyword.arg.as_ref().is_none_or(|arg| !options.contains(&arg.as_str())))
            .map(|keyword| checker.text(keyword.range()))
            .chain(wanted.iter().map(String::as_str))
            .collect();
        let fix = Fix::applicable_edit(
            Edit::range_replacement(format!("{}({})", checker.text(func), arguments.join(", ")), range),
            applicability(checker, class, explicit),
        );
        checker.report(
            rule,
            range,
            format!("Dataclass is missing {}; set `{}`", missing.join(", "), wanted.join(", ")),
            Some(fix),
        );
    }
}

// Each of these either fails at class creation or at runtime once the class is frozen and slotted.
struct SlotHazards {
    found: bool,
}

impl SlotHazards {
    const NAMES: [&str; 7] =
        ["__slots__", "__setattr__", "__delattr__", "__dict__", "__weakref__", "cached_property", "super"];
}

impl<'a> Visitor<'a> for SlotHazards {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if stmt.as_function_def_stmt().is_some_and(|function| Self::NAMES.contains(&function.name.as_str())) {
            self.found = true;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.found |= match expr {
            Expr::Attribute(ast::ExprAttribute { ctx: ExprContext::Store | ExprContext::Del, .. }) => true,
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => Self::NAMES.contains(&attr.as_str()),
            Expr::Name(ast::ExprName { id, .. }) => Self::NAMES.contains(&id.as_str()),
            _ => false,
        };
        visitor::walk_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::DataclassOptions, "SMP111.py" ; "dataclass_options")]
    #[test_case(Rule::DataclassKwOnly, "SMP114.py" ; "dataclass_kw_only")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
