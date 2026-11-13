use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn unregistered_marker(checker: &Checker) {
    // The marks pytest registers itself.
    const BUILTIN: &[&str] =
        &["skip", "skipif", "xfail", "parametrize", "usefixtures", "filterwarnings", "tryfirst", "trylast"];
    // Marks that common plugins register when they load, so the project's configuration never lists them.
    const PLUGIN: &[&str] = &[
        "anyio",
        "asyncio",
        "trio",
        "django_db",
        "urls",
        "ignore_template_errors",
        "timeout",
        "flaky",
        "benchmark",
        "order",
        "run",
        "first",
        "last",
        "dependency",
        "xdist_group",
        "httpx_mock",
        "respx",
        "vcr",
        "block_network",
        "default_cassette",
        "freeze_time",
        "repeat",
        "enable_socket",
        "disable_socket",
        "allow_hosts",
        "forked",
        "gen_test",
        "datafiles",
        "qt_no_exception_capture",
        "qt_log_level_fail",
        "qt_log_ignore",
        "limit_memory",
        "limit_leaks",
        "no_cover",
        "randomly_seed",
        "env",
        "mypy_testing",
        "subtests",
        "celery",
        "skip_if_unavailable",
        "thread_unsafe",
        "parallel_threads",
        "iterations",
    ];
    let Some(registered) = checker.settings.project.pytest.markers.as_ref() else {
        return;
    };
    let mut finder = Marks { checker, found: Vec::new() };
    finder.visit_body(checker.body());
    for attribute in finder.found {
        let name = attribute.attr.as_str();
        // A conftest's `pytest_configure` registers marks with `config.addinivalue_line("markers", "name: ...")`,
        // which spells the name in a string.
        if registered.contains(name)
            || BUILTIN.contains(&name)
            || PLUGIN.contains(&name)
            || checker.project().string_word_count(name) > 0
            || checker.project().is_mark_bound(name)
        {
            continue;
        }
        checker.report(
            Rule::UnregisteredMarker,
            attribute.attr.range(),
            format!(
                "pytest does not know the mark `{name}`, so it only warns, and a misspelled mark does nothing; register it under `markers` in the pytest configuration"
            ),
        );
    }
}

struct Marks<'a, 'b> {
    checker: &'a Checker<'b>,
    found: Vec<&'b ast::ExprAttribute>,
}

impl<'b> Visitor<'b> for Marks<'_, 'b> {
    fn visit_expr(&mut self, expr: &'b Expr) {
        if let Expr::Attribute(attribute) = expr
            && self.checker.semantic().resolves_to(&attribute.value, &["pytest", "mark"])
        {
            self.found.push(attribute);
        }
        visitor::walk_expr(self, expr);
    }
}
