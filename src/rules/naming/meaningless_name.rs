use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern};
use ruff_text_size::TextRange;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::naming::{NameSite, current, except_name, parameters, pattern_name};
use crate::syntax::{decorator_callee, last_segment, receiver};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    match expr {
        Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, range, .. }) if !is_model_field(walker) => {
            check(walker, current(walker), NameSite { text: id.as_str(), range: *range });
        }
        Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store, range, .. })
            if is_receiver_write(walker, value) =>
        {
            check(walker, current(walker), NameSite { text: attr.as_str(), range: *range });
        }
        _ => {}
    }
}

// A field of a model class, such as a dataclass, a pydantic model or a `TypedDict`, is also a key of the data it reads
// and writes, so its name is set by that data's schema.
fn is_model_field(walker: &Walker) -> bool {
    let [.., Scope::Class(class)] = walker.scopes() else {
        return false;
    };
    let decorated = class.decorator_list.iter().any(|decorator| {
        last_segment(decorator_callee(decorator)).is_some_and(|name| {
            matches!(name, "dataclass" | "define" | "frozen" | "mutable" | "s" | "attrs" | "sequence")
        })
    });
    decorated
        || class.bases().iter().any(|base| {
            last_segment(base).is_some_and(|name| {
                matches!(
                    name,
                    "BaseModel" | "BaseSettings" | "RootModel" | "SQLModel" | "TypedDict" | "NamedTuple" | "Struct"
                )
            })
        })
}

// `self.data = ...` in a method, or `cls.data = ...` in a classmethod, declares an attribute of the class, which the
// class's own name for it must describe. Writes through other objects name attributes the code does not choose.
fn is_receiver_write(walker: &Walker, value: &Expr) -> bool {
    let Expr::Name(ast::ExprName { id, .. }) = value else {
        return false;
    };
    matches!(walker.scopes(), [.., Scope::Class(_), Scope::Function(method)] if receiver(method) == Some(id.as_str()))
}

fn check<'a>(walker: &Walker<'_, 'a>, scope: TextRange, site: NameSite<'a>) {
    let bare = site.text.trim_matches('_');
    // `Foo` names a class, which this rule leaves alone, and `FOO` a constant, which it does not.
    let meaningless = &walker.checker.settings.project.knobs.meaningless_names;
    if meaningless.iter().any(|word| word == bare || word.to_uppercase() == bare)
        && walker.checker.state().reported_names().first_in_scope(scope, site.text)
    {
        walker.checker.report(
            Rule::MeaninglessName,
            site.range,
            format!("Meaningless name `{}`; name what it holds", site.text),
        );
    }
}

// A dunder method implements a protocol whose parameter names are fixed, as an override's are.
pub fn scope<'a>(walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
    parameters(walker, scope, |function| is_dunder(function.name.as_str()), check);
}

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    except_name(walker, handler, check);
}

pub fn pattern<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
    pattern_name(walker, pattern, check);
}
