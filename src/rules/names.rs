use std::collections::HashSet;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{dunder_all, is_class_like, rename_fix};
use crate::semantic::BindingKind;

pub fn single_letter_name(checker: &mut Checker) {
    let mut finder = SingleLetters { loop_target: false, scopes: vec![HashSet::new()], found: Vec::new() };
    finder.visit_body(checker.body());
    for (name, range) in finder.found {
        checker.report(
            Rule::SingleLetterName,
            range,
            format!("Single-letter name `{name}`; only loop and comprehension targets may use one"),
            None,
        );
    }
}

struct SingleLetters<'a> {
    loop_target: bool,
    scopes: Vec<HashSet<&'a str>>,
    found: Vec<(&'a str, TextRange)>,
}

impl<'a> Visitor<'a> for SingleLetters<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                self.check(function.name.as_str(), function.name.range);
                self.scopes.push(HashSet::new());
                self.check_parameters(&function.parameters);
                visitor::walk_stmt(self, stmt);
                self.scopes.pop();
                return;
            }
            Stmt::ClassDef(class) => {
                self.check(class.name.as_str(), class.name.range);
                self.scopes.push(HashSet::new());
                visitor::walk_stmt(self, stmt);
                self.scopes.pop();
                return;
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    if let Some(asname) = &alias.asname {
                        self.check(asname.as_str(), asname.range);
                    }
                }
            }
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => {
                self.visit_expr(iter);
                self.visit_loop_target(target);
                self.visit_body(body);
                self.visit_body(orelse);
                return;
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, range, .. }) => self.check(id.as_str(), *range),
            Expr::Lambda(ast::ExprLambda { parameters, .. }) => {
                self.scopes.push(HashSet::new());
                if let Some(parameters) = parameters {
                    self.check_parameters(parameters);
                }
                visitor::walk_expr(self, expr);
                self.scopes.pop();
                return;
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_comprehension(&mut self, comprehension: &'a ast::Comprehension) {
        self.visit_expr(&comprehension.iter);
        self.visit_loop_target(&comprehension.target);
        for condition in &comprehension.ifs {
            self.visit_expr(condition);
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(handler_node) = handler;
        if let Some(name) = &handler_node.name {
            self.check(name.as_str(), name.range);
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        if let Pattern::MatchAs(ast::PatternMatchAs { name: Some(name), .. })
        | Pattern::MatchStar(ast::PatternMatchStar { name: Some(name), .. }) = pattern
        {
            self.check(name.as_str(), name.range);
        }
        visitor::walk_pattern(self, pattern);
    }

    fn visit_type_param(&mut self, type_param: &'a ast::TypeParam) {
        let name = type_param.name();
        self.check(name.as_str(), name.range);
        visitor::walk_type_param(self, type_param);
    }
}

impl<'a> SingleLetters<'a> {
    fn check(&mut self, name: &'a str, range: TextRange) {
        let scope = self.scopes.last_mut().expect("module scope is never popped");
        if !self.loop_target && name.chars().count() == 1 && name != "_" && scope.insert(name) {
            self.found.push((name, range));
        }
    }

    fn check_parameters(&mut self, parameters: &'a ast::Parameters) {
        for parameter in parameters {
            self.check(parameter.name().as_str(), parameter.name().range);
        }
    }

    fn visit_loop_target(&mut self, target: &'a Expr) {
        let saved = std::mem::replace(&mut self.loop_target, true);
        self.visit_expr(target);
        self.loop_target = saved;
    }
}

pub fn module_variable_case(checker: &mut Checker) {
    const PYTEST_MAGIC: &[&str] = &["pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob"];
    let exported = dunder_all(checker.body());
    let mut found: Vec<(&str, TextRange, String)> = Vec::new();
    for (name, bindings) in &checker.semantic().bindings {
        if is_dunder(name)
            || is_class_like(name)
            || PYTEST_MAGIC.contains(name)
            || bindings
                .iter()
                .any(|binding| matches!(binding.kind, BindingKind::Import | BindingKind::Function | BindingKind::Class))
        {
            continue;
        }
        let private = if checker.is_imported_elsewhere(name) {
            name.starts_with('_')
        } else {
            !exported.as_ref().is_some_and(|names| names.contains(name))
        };
        let expected =
            format!("{}{}", if private { "_" } else { "" }, name.trim_start_matches('_').to_ascii_uppercase());
        if expected != *name {
            found.push((name, bindings[0].range, expected));
        }
    }
    found.sort_by_key(|(_, range, _)| range.start());
    for (name, range, expected) in found {
        let fix = rename_fix(checker, name, &expected);
        checker.report(
            Rule::ModuleVariableCase,
            range,
            format!("Module-scope variable `{name}` should be named `{expected}`"),
            fix,
        );
    }
}

pub fn unexported_public_name(checker: &mut Checker) {
    if checker.is_test_module() {
        return;
    }
    let Some(exported) = dunder_all(checker.body()) else {
        return;
    };
    let mut found = Vec::new();
    for stmt in checker.body() {
        let identifier = match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, decorator_list, .. })
            | Stmt::ClassDef(ast::StmtClassDef { name, decorator_list, .. })
                if decorator_list.is_empty() =>
            {
                name
            }
            _ => continue,
        };
        let name = identifier.as_str();
        if !name.starts_with('_')
            && !is_discovered_by_name(name)
            && !exported.contains(&name)
            && !checker.is_imported_elsewhere(name)
        {
            found.push(identifier);
        }
    }
    for identifier in found {
        let name = identifier.as_str();
        let private = format!("_{name}");
        let fix = rename_fix(checker, name, &private);
        checker.report(
            Rule::UnexportedPublicName,
            identifier.range(),
            format!("`{name}` is not in `__all__`; name it `{private}`"),
            fix,
        );
    }
}

fn is_discovered_by_name(name: &str) -> bool {
    name.starts_with("pytest_") || matches!(name, "load_tests" | "setup_module" | "teardown_module")
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::SingleLetterName, "SMP010.py" ; "single_letter_name")]
    #[test_case(Rule::ModuleVariableCase, "SMP102.py" ; "module_variable_case")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_noall.py" ; "module_variable_case_without_all")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_scopes.py" ; "module_variable_case_shadowed")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_rename.py" ; "module_variable_case_rename_across_scopes")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_call.py" ; "module_variable_case_dynamic_all")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_ann.py" ; "module_variable_case_annotated_all")]
    #[test_case(Rule::ModuleVariableCase, "SMP102_conflict.py" ; "module_variable_case_conflict")]
    #[test_case(Rule::UnexportedPublicName, "SMP103.py" ; "unexported_public_name")]
    #[test_case(Rule::UnexportedPublicName, "tests/test_SMP004.py" ; "unexported_public_name_test_module")]
    #[test_case(Rule::UnexportedPublicName, "SMP003_ok.py" ; "unexported_public_name_without_all")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
