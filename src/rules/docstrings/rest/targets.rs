use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::TextRange;

use super::attributes::{Attribute, attributes, is_field_call, skips_init, unsubscripted};
use super::{Docstring, has_attribute_bases};
use crate::lint::checker::Checker;
use crate::rules::docstring;
use crate::settings::DocstringStyle;
use crate::syntax::{Visibility, last_segment, module_statements};

pub(crate) enum Owner<'a> {
    Function { def: &'a ast::StmtFunctionDef, class: Option<&'a ast::StmtClassDef> },
    Class(&'a ast::StmtClassDef),
}

// A docstring the reStructuredText rules check, with what it documents and whether that is part of a library's public API, which
// must document all of its parameters, return value and exceptions; any other docstring must only be accurate.
pub(crate) struct Target<'a> {
    pub(crate) owner: Owner<'a>,
    pub(crate) docstring: Rc<Docstring<'a>>,
    pub(crate) public: bool,
    // Whether the docstring is the `__init__` docstring of a public class, which documents its constructor's
    // parameters as the class docstring may.
    pub(crate) public_constructor: bool,
    // For a class, the docstring of its `__init__`, where some projects document the constructor's parameters instead.
    pub(crate) constructor: Option<Rc<Docstring<'a>>>,
    // For a class, its attributes, found when a rule first asks for them.
    attributes: OnceCell<Vec<Attribute<'a>>>,
}

impl<'a> Target<'a> {
    pub(crate) fn name(&self) -> &'a str {
        match self.owner {
            Owner::Function { def, .. } => def.name.as_str(),
            Owner::Class(class) => class.name.as_str(),
        }
    }

    // The parameters the docstring may document: a function's, or the constructor's for a class, `None` when the class
    // takes them from elsewhere, such as a base class.
    pub(crate) fn parameters(&self) -> Option<Vec<Parameter<'a>>> {
        match self.owner {
            Owner::Function { def, class } => Some(parameters(def, class.is_some())),
            Owner::Class(class) => class_parameters(class),
        }
    }

    // The attributes of a class, none for a function.
    pub(crate) fn attributes(&self, checker: &Checker) -> &[Attribute<'a>] {
        match self.owner {
            Owner::Class(class) => self.attributes.get_or_init(|| attributes(checker, class)),
            Owner::Function { .. } => &[],
        }
    }

    pub(crate) fn takes_keywords(&self) -> bool {
        match self.owner {
            Owner::Function { def, .. } => def.parameters.kwarg.is_some(),
            Owner::Class(class) => constructor(class).is_some_and(|init| init.parameters.kwarg.is_some()),
        }
    }
}

// Built once per file; rules read them through `Checker::rest_targets`.
pub(crate) fn collect_targets<'a>(checker: &Checker<'a>) -> Vec<Target<'a>> {
    if checker.settings.project.docstring_style != Some(DocstringStyle::Rest) {
        return Vec::new();
    }
    let api = checker.public_api();
    let methods: HashMap<TextRange, &ast::StmtClassDef> = checker
        .class_defs()
        .iter()
        .flat_map(|class| {
            module_statements(&class.body).filter_map(Stmt::as_function_def_stmt).map(move |def| (def.range, *class))
        })
        .collect();
    let mut targets: Vec<Target> = checker
        .function_defs()
        .iter()
        .filter_map(|def| {
            let docstring = Rc::new(Docstring::parse(checker, docstring(&def.body)?));
            let class = methods.get(&def.range).copied();
            let public_constructor =
                def.name.as_str() == "__init__" && class.is_some_and(|class| api.is_public(class.range));
            let owner = Owner::Function { def, class };
            let public = api.is_public(def.range);
            Some(Target {
                owner,
                docstring,
                public,
                public_constructor,
                constructor: None,
                attributes: OnceCell::new(),
            })
        })
        .collect();
    // By the range of the docstring, so the class shares the one its `__init__` target holds.
    let parsed: HashMap<TextRange, Rc<Docstring>> =
        targets.iter().map(|target| (target.docstring.range(), Rc::clone(&target.docstring))).collect();
    let classes = checker.class_defs().iter().filter_map(|class| {
        let constructor = constructor(class)
            .and_then(|init| docstring(&init.body))
            .and_then(|literal| parsed.get(&literal.range))
            .cloned();
        let docstring = Rc::new(Docstring::parse(checker, docstring(&class.body)?));
        let public = api.is_public(class.range);
        let owner = Owner::Class(class);
        Some(Target { owner, docstring, public, public_constructor: false, constructor, attributes: OnceCell::new() })
    });
    targets.extend(classes);
    targets.sort_by_key(|target| target.docstring.range().start());
    targets
}

pub(crate) struct Parameter<'a> {
    pub(crate) name: &'a str,
    pub(crate) annotated: bool,
}

// A function's parameters in order, without the instance or class a method receives first and without names made of
// underscores only, which take a value nobody reads.
fn parameters(def: &ast::StmtFunctionDef, is_method: bool) -> Vec<Parameter<'_>> {
    let receives = is_method
        && !def.decorator_list.iter().any(|decorator| last_segment(&decorator.expression) == Some("staticmethod"));
    let parameters = &def.parameters;
    let positional = parameters.posonlyargs.iter().chain(&parameters.args).map(|parameter| &parameter.parameter);
    let all = positional
        .chain(parameters.vararg.as_deref())
        .chain(parameters.kwonlyargs.iter().map(|parameter| &parameter.parameter))
        .chain(parameters.kwarg.as_deref());
    all.skip(usize::from(receives))
        .filter(|parameter| !parameter.name.as_str().chars().all(|char| char == '_'))
        .map(|parameter| Parameter { name: parameter.name.as_str(), annotated: parameter.annotation.is_some() })
        .collect()
}

// The `__init__` that runs: the last one, since `@overload` stubs come before the implementation.
fn constructor(class: &ast::StmtClassDef) -> Option<&ast::StmtFunctionDef> {
    module_statements(&class.body)
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|def| {
            def.name.as_str() == "__init__"
                && !def.decorator_list.iter().any(|decorator| last_segment(&decorator.expression) == Some("overload"))
        })
        .last()
}

// A class's constructor parameters: its `__init__`'s, or the fields a dataclass or attrs class turns into them, those
// annotated in its body and those an `attr.ib()` or `field()` call declares, without `ClassVar`s, the `KW_ONLY`
// marker and fields that set `init=False`. `None` when they come from elsewhere: a base class, or a class that is not
// generated.
fn class_parameters(class: &ast::StmtClassDef) -> Option<Vec<Parameter<'_>>> {
    if let Some(init) = constructor(class) {
        return Some(parameters(init, true));
    }
    // A generator called with `init=False` writes no constructor, so the class keeps `object`'s.
    let generated = class.decorator_list.iter().any(|decorator| {
        let call = decorator.expression.as_call_expr();
        let name = last_segment(call.map_or(&decorator.expression, |call| &call.func));
        !call.is_some_and(skips_init)
            && matches!(name, Some("dataclass" | "define" | "frozen" | "mutable" | "attrs" | "s"))
    }) || class.bases().iter().any(|base| last_segment(base) == Some("NamedTuple"));
    if !generated || has_attribute_bases(class) {
        return None;
    }
    let fields = module_statements(&class.body).filter_map(|stmt| match stmt {
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) => {
            let marker = matches!(last_segment(unsubscripted(annotation)), Some("ClassVar" | "KW_ONLY"));
            let skipped = value.as_deref().and_then(Expr::as_call_expr).is_some_and(skips_init);
            target.as_name_expr().filter(|_| !marker && !skipped).map(|name| (name, true))
        }
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match targets.as_slice() {
            [Expr::Name(name)] if is_field_call(value) && !value.as_call_expr().is_some_and(skips_init) => {
                Some((name, false))
            }
            _ => None,
        },
        _ => None,
    });
    Some(
        fields
            .filter(|(name, _)| !name.id.as_str().chars().all(|char| char == '_'))
            .map(|(name, annotated)| Parameter { name: name.id.as_str(), annotated })
            .collect(),
    )
}

// Whether a parameter or attribute name is internal. A leading underscore marks one, but not a dunder, as `__wrapped__`,
// and not a parameter named with two underscores, which marks it positional-only, as in typeshed.
pub(crate) fn is_internal(name: &str, parameter: bool) -> bool {
    Visibility::of(name) == Visibility::Private && !(parameter && name.starts_with("__"))
}
