use ruff_python_ast::{self as ast, Expr};

use crate::syntax::last_segment;

use super::visitor::FactVisitor;

// A target that starts with the interpolation names no module the run can find.
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

pub(super) fn named_attribute_call(func: &Expr) -> bool {
    match last_segment(func) {
        Some("setattr" | "delattr") => true,
        Some("object") => {
            matches!(func, Expr::Attribute(ast::ExprAttribute { value, .. }) if last_segment(value) == Some("patch"))
        }
        _ => false,
    }
}

fn patch_object(func: &Expr) -> bool {
    matches!(func, Expr::Attribute(ast::ExprAttribute { attr, value, .. })
        if attr.as_str() == "object" && last_segment(value) == Some("patch"))
}

impl FactVisitor<'_> {
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

    fn receivers(&self, target: &Expr) -> Vec<String> {
        match target.as_name_expr().and_then(|name| self.loop_modules.get(name.id.as_str())) {
            Some(modules) => modules.clone(),
            None => self.qualified(target).into_iter().collect(),
        }
    }

    // The receiver's name does not matter, so any object counts.
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
