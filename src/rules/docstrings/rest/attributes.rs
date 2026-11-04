use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::checker::Checker;
use crate::rules::docstrings::{is_documented_variable, module_statements_with_next};
use crate::rules::{for_each_body, last_segment, module_statements};

// An attribute of a class as autodoc documents it: one its body assigns, or one a method sets on `self`.
pub(crate) struct Attribute<'a> {
    pub(crate) name: &'a str,
    // Whether it belongs to the class rather than its instances: a `ClassVar`, or a name the body assigns without an
    // annotation; and whether instances hold it, as an annotated field or a name a method sets on `self`, which a class
    // default may be too.
    pub(crate) class_var: bool,
    pub(crate) on_instance: bool,
    // Whether a `#:` comment or a string right after it documents it, as autodoc reads.
    pub(crate) documented: bool,
}

pub(crate) fn attributes<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Vec<Attribute<'a>> {
    let mut found: Vec<Attribute> = Vec::new();
    let mut add = |name: &'a str, class_var: bool, documented: bool| match found
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        Some(attribute) => {
            attribute.documented |= documented;
            attribute.on_instance |= !class_var;
        }
        None => found.push(Attribute { name, class_var, on_instance: !class_var, documented }),
    };
    for (stmt, next) in module_statements_with_next(&class.body) {
        let documented = is_documented_variable(checker, stmt, next);
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) => {
                let annotation =
                    annotation.as_subscript_expr().map_or(annotation.as_ref(), |subscript| &subscript.value);
                if let Some(name) = target.as_name_expr() {
                    add(name.id.as_str(), last_segment(annotation) == Some("ClassVar"), documented);
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                // An attrs or dataclass field declared by a call holds a value per instance.
                let field = value
                    .as_call_expr()
                    .is_some_and(|call| matches!(last_segment(&call.func), Some("ib" | "attrib" | "field")));
                for name in targets.iter().filter_map(Expr::as_name_expr) {
                    add(name.id.as_str(), !field, documented);
                }
            }
            _ => {}
        }
    }
    for method in module_statements(&class.body).filter_map(Stmt::as_function_def_stmt) {
        // A static method receives nothing; a class method receives the class, whose attributes are class variables.
        let decorators: Vec<&str> =
            method.decorator_list.iter().filter_map(|decorator| last_segment(&decorator.expression)).collect();
        if decorators.contains(&"staticmethod") {
            continue;
        }
        let on_class = decorators.contains(&"classmethod");
        let parameters = &method.parameters;
        let Some(receiver) =
            parameters.posonlyargs.iter().chain(&parameters.args).next().map(|parameter| parameter.name().as_str())
        else {
            continue;
        };
        for_each_body(&method.body, &mut |body, _| {
            for (index, stmt) in body.iter().enumerate() {
                let documented = || is_documented_variable(checker, stmt, body.get(index + 1));
                // A frozen dataclass sets its own attributes with `object.__setattr__(self, "name", value)`.
                if let Some(name) = set_through_object(stmt, receiver) {
                    add(name, on_class, documented());
                }
                let mut targets: Vec<&Expr> = match stmt {
                    Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().collect(),
                    Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })
                    | Stmt::AugAssign(ast::StmtAugAssign { target, .. })
                    | Stmt::For(ast::StmtFor { target, .. }) => vec![target.as_ref()],
                    Stmt::With(ast::StmtWith { items, .. }) => {
                        items.iter().filter_map(|item| item.optional_vars.as_deref()).collect()
                    }
                    _ => Vec::new(),
                };
                // `self.left, self.right = pair` sets both, in the order written.
                let mut next = 0;
                while let Some(&target) = targets.get(next) {
                    next += 1;
                    match target {
                        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                            targets.extend(elts);
                        }
                        Expr::Starred(ast::ExprStarred { value, .. }) => targets.push(value),
                        Expr::Attribute(attribute)
                            if attribute.value.as_name_expr().is_some_and(|name| name.id.as_str() == receiver) =>
                        {
                            add(attribute.attr.as_str(), on_class, documented());
                        }
                        _ => {}
                    }
                }
            }
        });
    }
    found
}

// The attribute an `object.__setattr__(receiver, "name", value)` call sets.
fn set_through_object<'a>(stmt: &'a Stmt, receiver: &str) -> Option<&'a str> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr()?;
    let is_object = attribute.value.as_name_expr().is_some_and(|name| name.id.as_str() == "object");
    match call.arguments.args.as_ref() {
        [Expr::Name(target), Expr::StringLiteral(name), ..]
            if is_object && attribute.attr.as_str() == "__setattr__" && target.id.as_str() == receiver =>
        {
            Some(name.value.to_str())
        }
        _ => None,
    }
}

// Whether the class takes attributes from a base that may define them, as a field list cannot tell.
pub(crate) fn has_attribute_bases(class: &ast::StmtClassDef) -> bool {
    class.bases().iter().any(|base| {
        let base = base.as_subscript_expr().map_or(base, |subscript| &subscript.value);
        !matches!(last_segment(base), Some("object" | "ABC" | "Protocol" | "Generic" | "NamedTuple" | "TypedDict"))
    })
}
