use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::scoped_block::identifier_uses;
use crate::syntax::for_each_child_body;

// Python 3.15 adds the `sentinel` builtin, whose repr and pickle carry the name the placeholder is bound to.
pub fn up_object_sentinel(checker: &Checker) {
    if identifier_uses(checker, TextRange::up_to(checker.source.text_len()), "sentinel") > 0 {
        return;
    }
    placeholders(checker, checker.body(), &Enclosing::default(), true);
}

// The classes a body sits in, by their dotted name, and whether a function encloses them.
#[derive(Clone, Default)]
struct Enclosing {
    class: Option<String>,
    function: bool,
}

impl Enclosing {
    fn nested(&self, class: &str) -> Self {
        let class = match &self.class {
            Some(outer) => format!("{outer}.{class}"),
            None => class.to_owned(),
        };
        Self { class: Some(class), function: self.function }
    }

    fn in_function(&self) -> Self {
        Self { class: self.class.clone(), function: true }
    }

    fn qualified(&self, name: &str) -> String {
        match &self.class {
            Some(class) => format!("{class}.{name}"),
            None => name.to_owned(),
        }
    }
}

// Only module and class bodies hold the placeholders the rule means, so `direct` marks those; a function's local is not
// reachable by name.
fn placeholders(checker: &Checker, body: &[Stmt], enclosing: &Enclosing, direct: bool) {
    for stmt in body {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if direct => {
                if let [Expr::Name(target)] = targets.as_slice()
                    && is_bare_object(checker, value)
                    && !checker.semantic().is_rebound(target.id.as_str())
                {
                    report(checker, target.id.as_str(), value, enclosing);
                }
            }
            Stmt::ClassDef(class) => {
                placeholders(checker, &class.body, &enclosing.nested(class.name.as_str()), true);
            }
            Stmt::FunctionDef(function) => placeholders(checker, &function.body, &enclosing.in_function(), false),
            _ => for_each_child_body(stmt, &mut |child| placeholders(checker, child, enclosing, false)),
        }
    }
}

fn is_bare_object(checker: &Checker, value: &Expr) -> bool {
    matches!(value, Expr::Call(ast::ExprCall { func, arguments, .. })
        if arguments.args.is_empty()
            && arguments.keywords.is_empty()
            && checker.semantic().is_builtin(func, "object"))
}

fn report(checker: &Checker, name: &str, value: &Expr, enclosing: &Enclosing) {
    let qualified = enclosing.qualified(name);
    // A class inside a function is not reached by its qualified name, which pickle cannot find, so it gets no fix.
    let fix = (!enclosing.function).then(|| {
        // `sentinel` changes what the name reprs as and how it pickles, which a program can observe.
        Fix::unsafe_edit(Edit::range_replacement(format!("sentinel(\"{qualified}\")"), value.range()))
    });
    checker.report_fix(
        Rule::ObjectSentinel,
        value.range(),
        format!("`{name}` is a bare `object()`, whose repr is an address; `sentinel(\"{qualified}\")` names it"),
        fix,
        "Create it with `sentinel`",
    );
}
