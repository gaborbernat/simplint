use rustc_hash::FxHashMap;
use std::cell::OnceCell;
use std::rc::Rc;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::TextRange;

use super::attributes::{Attribute, annotation_name, attributes, is_field_call, skips_init};
use super::{Docstring, has_attribute_bases};
use crate::lint::checker::Checker;
use crate::rules::public_api::Need;
use crate::settings::DocstringStyle;
use crate::syntax::{Visibility, docstring, is_overload, last_segment, module_statements, receiver};

pub(crate) enum Owner<'a> {
    Function { def: &'a ast::StmtFunctionDef, class: Option<&'a ast::StmtClassDef> },
    Class(&'a ast::StmtClassDef),
}

// A docstring the reStructuredText rules check, with what it documents. A library's public API must document all of its
// parameters, return value and exceptions; any other docstring must only be accurate.
pub(crate) struct Target<'a> {
    pub(crate) owner: Owner<'a>,
    pub(crate) docstring: Rc<Docstring<'a>>,
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

    // Whether the definition is part of the library's public API. Only the rules that ask for it pay for the public API,
    // which reads the index of the whole run.
    pub(crate) fn public(&self, checker: &Checker) -> bool {
        checker.state().public_api(checker).is_public(self.definition(), Need::Docs)
    }

    // Whether the docstring is the `__init__` docstring of a public class, which documents its constructor's parameters
    // as the class docstring may.
    pub(crate) fn public_constructor(&self, checker: &Checker) -> bool {
        match self.owner {
            Owner::Function { def, class: Some(class) } => {
                def.name.as_str() == "__init__"
                    && checker.state().public_api(checker).is_public(class.range, Need::Docs)
            }
            Owner::Function { class: None, .. } | Owner::Class(_) => false,
        }
    }

    fn definition(&self) -> TextRange {
        match self.owner {
            Owner::Function { def, .. } => def.range,
            Owner::Class(class) => class.range,
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

// Built once per file; rules read them through `Checker::rst_targets`.
pub(crate) fn collect_targets<'a>(checker: &Checker<'a>) -> Vec<Target<'a>> {
    if checker.settings.project.docstring_style != Some(DocstringStyle::Rst) {
        return Vec::new();
    }
    let methods: FxHashMap<TextRange, &ast::StmtClassDef> = checker
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
            let owner = Owner::Function { def, class };
            Some(Target { owner, docstring, constructor: None, attributes: OnceCell::new() })
        })
        .collect();
    // By the range of the docstring, so the class shares the one its `__init__` target holds.
    let parsed: FxHashMap<TextRange, Rc<Docstring>> =
        targets.iter().map(|target| (target.docstring.range(), Rc::clone(&target.docstring))).collect();
    let classes = checker.class_defs().iter().filter_map(|class| {
        let constructor = constructor(class)
            .and_then(|init| docstring(&init.body))
            .and_then(|literal| parsed.get(&literal.range))
            .cloned();
        let docstring = Rc::new(Docstring::parse(checker, docstring(&class.body)?));
        let owner = Owner::Class(class);
        Some(Target { owner, docstring, constructor, attributes: OnceCell::new() })
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
    let received = if is_method { receiver(def) } else { None };
    let parameters = &def.parameters;
    let positional = parameters.posonlyargs.iter().chain(&parameters.args).map(|parameter| &parameter.parameter);
    let all = positional
        .chain(parameters.vararg.as_deref())
        .chain(parameters.kwonlyargs.iter().map(|parameter| &parameter.parameter))
        .chain(parameters.kwarg.as_deref());
    all.filter(|parameter| Some(parameter.name.as_str()) != received)
        .filter(|parameter| !underscores_only(parameter.name.as_str()))
        .map(|parameter| Parameter { name: parameter.name.as_str(), annotated: parameter.annotation.is_some() })
        .collect()
}

fn underscores_only(name: &str) -> bool {
    name.chars().all(|char| char == '_')
}

// The `__init__` that runs: the last one, since `@overload` stubs come before the implementation.
fn constructor(class: &ast::StmtClassDef) -> Option<&ast::StmtFunctionDef> {
    module_statements(&class.body)
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|def| def.name.as_str() == "__init__" && !is_overload(&def.decorator_list))
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
    let generator = class.decorator_list.iter().find_map(|decorator| {
        let call = decorator.expression.as_call_expr();
        let name = last_segment(call.map_or(&decorator.expression, |call| &call.func))?;
        let generates = matches!(name, "dataclass" | "define" | "frozen" | "mutable" | "attrs" | "s");
        (generates && !call.is_some_and(skips_init)).then_some(name)
    });
    let namedtuple = class.bases().iter().any(|base| last_segment(base) == Some("NamedTuple"));
    if (generator.is_none() && !namedtuple) || has_attribute_bases(class) {
        return None;
    }
    let attrs = generator.is_some_and(|name| name != "dataclass");
    let fields = module_statements(&class.body).filter_map(|stmt| match stmt {
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) => {
            let marker = matches!(annotation_name(annotation), Some("ClassVar" | "KW_ONLY"));
            let skipped = value.as_deref().and_then(Expr::as_call_expr).is_some_and(skips_init);
            target.as_name_expr().filter(|_| !marker && !skipped).map(|name| (name, value.as_deref(), true))
        }
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match targets.as_slice() {
            [Expr::Name(name)] if is_field_call(value) && !value.as_call_expr().is_some_and(skips_init) => {
                Some((name, Some(value.as_ref()), false))
            }
            _ => None,
        },
        _ => None,
    });
    fields
        .filter(|(name, _, _)| !underscores_only(name.id.as_str()))
        .map(|(name, value, annotated)| Some(Parameter { name: init_name(name, value, attrs)?, annotated }))
        .collect()
}

// The name of a field's constructor parameter: attrs takes it from the field's `alias`, or from the field's name
// without its leading underscores; a dataclass keeps the field's name. An alias that is not a string literal names the
// parameter in a way the class does not show, so the class's parameters are unknown.
fn init_name<'a>(name: &'a ast::ExprName, value: Option<&'a Expr>, attrs: bool) -> Option<&'a str> {
    if !attrs {
        return Some(name.id.as_str());
    }
    let alias = value.and_then(Expr::as_call_expr).and_then(|call| call.arguments.find_keyword("alias"));
    match alias {
        None => Some(name.id.trim_start_matches('_')),
        Some(keyword) => keyword.value.as_string_literal_expr().map(|literal| literal.value.to_str()),
    }
}

// Whether a parameter or attribute name is internal. A leading underscore marks one, but not a dunder, as `__wrapped__`,
// and not a parameter named with two underscores, which marks it positional-only, as in typeshed.
pub(crate) fn is_internal(name: &str, parameter: bool) -> bool {
    Visibility::of(name) == Visibility::Private && !(parameter && name.starts_with("__"))
}
