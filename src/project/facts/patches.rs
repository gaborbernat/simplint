use ruff_python_ast::{self as ast, Expr};

use crate::syntax::last_segment;

use super::visitor::FactVisitor;

// The module a `mock.patch` target f-string names before its first interpolated name, `pkg.report` for
// `f"pkg.report.{name}"`. A target that starts with the interpolation names no module the run can find.
fn computed_patch_module(target: &Expr) -> Option<&str> {
    let Expr::FString(fstring) = target else {
        return None;
    };
    let mut elements = fstring.value.elements();
    let Some(ast::InterpolatedStringElement::Literal(literal)) = elements.next() else {
        return None;
    };
    if !matches!(elements.next(), Some(ast::InterpolatedStringElement::Interpolation(_))) {
        return None;
    }
    literal.value.rsplit_once('.').map(|(module, _)| module)
}

// Whether a call takes the object an attribute belongs to first and the attribute's name second, as `setattr`,
// `monkeypatch.delattr` or `patch.object` does.
pub(super) fn named_attribute_call(func: &Expr) -> bool {
    match last_segment(func) {
        Some("setattr" | "delattr") => true,
        Some("object") => {
            matches!(func, Expr::Attribute(ast::ExprAttribute { value, .. }) if last_segment(value) == Some("patch"))
        }
        _ => false,
    }
}

// Whether a call is `patch.object`, as `mock.patch.object` or `mocker.patch.object` is.
fn patch_object(func: &Expr) -> bool {
    matches!(func, Expr::Attribute(ast::ExprAttribute { attr, value, .. })
        if attr.as_str() == "object" && last_segment(value) == Some("patch"))
}

impl FactVisitor<'_> {
    // The module and the names a call sets through a literal or a loop over string literals, as `setattr(module, "NAME")`
    // or `patch.object(module, name)` does.
    pub(super) fn named_names(&self, arguments: &ast::Arguments) -> Vec<(String, Vec<String>)> {
        match &*arguments.args {
            [target, Expr::StringLiteral(name), ..] => self
                .receivers(target)
                .into_iter()
                .map(|module| (module, vec![name.value.to_str().to_owned()]))
                .collect(),
            [target, Expr::Name(name), ..] => {
                let Some(strings) = self.loop_strings.get(name.id.as_str()) else {
                    return Vec::new();
                };
                self.receivers(target).into_iter().map(|module| (module, strings.clone())).collect()
            }
            _ => Vec::new(),
        }
    }

    // The modules a call's receiver stands for: the module it imports, or each module a loop over a literal of modules
    // binds it to.
    fn receivers(&self, target: &Expr) -> Vec<String> {
        match target.as_name_expr().and_then(|name| self.loop_modules.get(name.id.as_str())) {
            Some(modules) => modules.clone(),
            None => self.qualified(target).into_iter().collect(),
        }
    }

    // The module and the names a call writes on it: a `<receiver>.setattr` call, a `patch.object` call, or a `mock.patch`
    // of a dotted string, which pytest and `unittest.mock` split at the last dot. The receiver's name does not matter, so
    // any object counts.
    pub(super) fn module_writes(
        func: &Expr,
        arguments: &ast::Arguments,
        named: Vec<(String, Vec<String>)>,
    ) -> Vec<(String, Vec<String>)> {
        let setattr = matches!(func, Expr::Attribute(ast::ExprAttribute { attr, .. }) if attr.as_str() == "setattr");
        match &*arguments.args {
            [Expr::StringLiteral(path), ..] if setattr || last_segment(func) == Some("patch") => {
                let Some((module, name)) = path.value.to_str().rsplit_once('.') else {
                    return Vec::new();
                };
                vec![(module.to_owned(), vec![name.to_owned()])]
            }
            _ if setattr || patch_object(func) => named,
            _ => Vec::new(),
        }
    }

    // Records the module a write under a name the run cannot know reaches: `setattr(module, name, value)` with a name
    // that is neither a literal nor a loop over string literals, or a `mock.patch` target f-string spelling the module.
    pub(super) fn computed_write(&mut self, func: &Expr, arguments: &ast::Arguments) {
        let module = match &*arguments.args {
            [target, name, ..]
                if named_attribute_call(func)
                    && !name.is_string_literal_expr()
                    && !name.as_name_expr().is_some_and(|name| self.loop_strings.contains_key(name.id.as_str())) =>
            {
                self.qualified(target)
            }
            [target, ..] if last_segment(func) == Some("patch") => computed_patch_module(target).map(str::to_owned),
            _ => None,
        };
        self.computed_writes.extend(module);
    }
}
