use rustc_hash::FxHashMap;
use std::cell::OnceCell;
use std::rc::Rc;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::TextRange;

use super::attributes::{Attribute, attributes, is_field_call, skips_init};
use super::has_attribute_bases;
use super::parse::Docstring;
use crate::lint::checker::Checker;
use crate::rules::public_api::Need;
use crate::settings::DocstringStyle;
use crate::syntax::{Declares, Visibility, docstring, is_overload, last_segment, module_statements, receiver};

pub(crate) enum Owner<'a> {
    Function { def: &'a ast::StmtFunctionDef, class: Option<&'a ast::StmtClassDef> },
    Class(&'a ast::StmtClassDef),
}

pub(crate) struct Target<'a> {
    pub(crate) owner: Owner<'a>,
    pub(crate) docstring: Rc<Docstring<'a>>,
    // Constructor parameter descriptions can appear in either docstring.
    pub(crate) constructor: Option<Rc<Docstring<'a>>>,
    attributes: OnceCell<Vec<Attribute<'a>>>,
}

pub(crate) struct Parameter<'a> {
    pub(crate) name: &'a str,
    pub(crate) annotated: bool,
}

impl<'a> Target<'a> {
    pub(crate) fn name(&self) -> &'a str {
        match self.owner {
            Owner::Function { def, .. } => def.name.as_str(),
            Owner::Class(class) => class.name.as_str(),
        }
    }

    // Inherited and otherwise unknown constructor signatures have no parameter list here.
    pub(crate) fn parameters(&self) -> Option<Vec<Parameter<'a>>> {
        match self.owner {
            Owner::Function { def, class } => Some(parameters(def, class.is_some())),
            Owner::Class(class) => class_parameters(class),
        }
    }

    // The project index is needed only for rules that require public API documentation.
    pub(crate) fn public(&self, checker: &Checker) -> bool {
        checker.state().public_api(checker).is_public(self.definition(), Need::Docs)
    }

    fn definition(&self) -> TextRange {
        match self.owner {
            Owner::Function { def, .. } => def.range,
            Owner::Class(class) => class.range,
        }
    }

    pub(crate) fn public_constructor(&self, checker: &Checker) -> bool {
        match self.owner {
            Owner::Function { def, class: Some(class) } => {
                def.name.as_str() == "__init__"
                    && checker.state().public_api(checker).is_public(class.range, Need::Docs)
            }
            Owner::Function { class: None, .. } | Owner::Class(_) => false,
        }
    }

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
            Some(Target {
                owner: Owner::Function { def, class: methods.get(&def.range).copied() },
                docstring: Rc::new(Docstring::parse(checker, docstring(&def.body)?)),
                constructor: None,
                attributes: OnceCell::new(),
            })
        })
        .collect();
    // Reuse the parsed constructor docstring when checking the class.
    let parsed: FxHashMap<TextRange, Rc<Docstring>> =
        targets.iter().map(|target| (target.docstring.range(), Rc::clone(&target.docstring))).collect();
    targets.extend(checker.class_defs().iter().filter_map(|class| {
        let constructor = constructor(class)
            .and_then(|init| docstring(&init.body))
            .and_then(|literal| parsed.get(&literal.range))
            .cloned();
        Some(Target {
            owner: Owner::Class(class),
            docstring: Rc::new(Docstring::parse(checker, docstring(&class.body)?)),
            constructor,
            attributes: OnceCell::new(),
        })
    }));
    targets.sort_by_key(|target| target.docstring.range().start());
    targets
}

// A method receiver is implicit, and underscore-only names conventionally mark ignored arguments.
fn parameters(def: &ast::StmtFunctionDef, is_method: bool) -> Vec<Parameter<'_>> {
    let received = if is_method { receiver(def) } else { None };
    let parameters = &def.parameters;
    let all = parameters
        .posonlyargs
        .iter()
        .chain(&parameters.args)
        .map(|parameter| &parameter.parameter)
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

// Inherited fields can change a generated constructor's signature.
fn class_parameters(class: &ast::StmtClassDef) -> Option<Vec<Parameter<'_>>> {
    if let Some(init) = constructor(class) {
        return Some(parameters(init, true));
    }
    // With `init=False` the decorator leaves constructor selection to Python.
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
            let marker = matches!(Declares::of(annotation), Declares::ClassVar | Declares::KwOnly);
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

fn constructor(class: &ast::StmtClassDef) -> Option<&ast::StmtFunctionDef> {
    module_statements(&class.body)
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|def| def.name.as_str() == "__init__" && !is_overload(&def.decorator_list))
        .last()
}

// attrs strips leading underscores unless the field supplies an alias. A computed alias leaves the parameter name
// unknown.
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

// Historical typing conventions use two leading underscores for positional-only parameters.
pub(crate) fn is_internal(name: &str, parameter: bool) -> bool {
    Visibility::of(name) == Visibility::Private && !(parameter && name.starts_with("__"))
}
