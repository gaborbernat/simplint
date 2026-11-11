use ruff_python_ast::{self as ast, Expr};

use crate::syntax::has_decorator;

use super::visitor::FactVisitor;

pub(in crate::project) const PYTEST_PLUGINS_REQUEST: &str = "pytest_plugins";

pub(super) fn is_pytest_plugins(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| name.id.as_str() == "pytest_plugins")
}

impl FactVisitor<'_> {
    pub(super) fn request_parameters(&mut self, function: &ast::StmtFunctionDef) {
        let parameters: Vec<String> =
            function.parameters.iter().map(|parameter| parameter.name().to_string()).collect();
        if function.name.starts_with("test") || has_decorator(&function.decorator_list, &["fixture"]) {
            self.pytest_requests.extend(parameters.clone());
        }
        self.fixture_requests.extend(parameters);
    }

    // A plugin may request fixtures of the run by name. The index records that as a request under `pytest_plugins`,
    // which no fixture can take; pytest's own plugins request none.
    pub(super) fn load_plugins(&mut self, value: &Expr) {
        let plugins: Vec<&Expr> = match value {
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().collect(),
            single => vec![single],
        };
        if !plugins.iter().all(|plugin| {
            plugin.as_string_literal_expr().is_some_and(|name| {
                let name = name.value.to_str();
                name == "pytester" || name.starts_with("_pytest")
            })
        }) {
            self.fixture_requests.insert(PYTEST_PLUGINS_REQUEST.to_owned());
        }
    }
}
