mod function_docstring;
mod long_docstring_paragraph;
pub(crate) mod rst;
mod rst_attribute_kind;
mod rst_duplicate_param;
mod rst_empty_field;
mod rst_field_syntax;
mod rst_inline_markup;
mod rst_missing_attribute;
mod rst_missing_param;
mod rst_missing_raises;
mod rst_param_order;
mod rst_private_field;
mod rst_restating_field;
mod rst_returns;
mod rst_type_field;
mod rst_unknown_attribute;
mod rst_unknown_param;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use long_docstring_paragraph::long_docstring_paragraph;
pub(crate) use rst_attribute_kind::rst_attribute_kind;
pub(crate) use rst_duplicate_param::rst_duplicate_param;
pub(crate) use rst_empty_field::rst_empty_field;
pub(crate) use rst_field_syntax::rst_field_syntax;
pub(crate) use rst_inline_markup::rst_inline_markup;
pub(crate) use rst_missing_attribute::rst_missing_attribute;
pub(crate) use rst_missing_param::rst_missing_param;
pub(crate) use rst_missing_raises::rst_missing_raises;
pub(crate) use rst_param_order::rst_param_order;
pub(crate) use rst_private_field::rst_private_field;
pub(crate) use rst_restating_field::rst_restating_field;
pub(crate) use rst_returns::rst_returns;
pub(crate) use rst_type_field::rst_type_field;
pub(crate) use rst_unknown_attribute::rst_unknown_attribute;
pub(crate) use rst_unknown_param::rst_unknown_param;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use std::path::Path;

use ruff_diagnostics::Edit;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::project::{Exposure, FileIndex};
use crate::rules::comments::is_tool_directive;
use crate::rules::edits::{Blanks, statement_deletion};
use crate::rules::{indentation, is_alone_on_lines};
use crate::syntax::{Exports, Visibility, decorator_names, is_overload, module_statements};

enum ApiKind {
    Function,
    Class,
    Method,
    Property,
    Variable,
}

struct ApiItem<'a> {
    stmt: &'a Stmt,
    name: String,
    range: TextRange,
    kind: ApiKind,
    // The statement after a variable's in its block, where its docstring goes.
    next: Option<&'a Stmt>,
    // The file that re-exports the item to users, when its own module does not offer it.
    via: Option<&'a Path>,
    // Whether the docstring rules require docs for the item: not when only a signature of an exposed name reaches it.
    docs: bool,
}

// The public API of a library is `FileIndex::library_exposure`, the names its users can import, with the public members
// of those classes. An application has none. What the docstring rules add is documentation: autodoc-documented
// definitions, whose names only the checker's module path makes qualified, and the members that need no docstring.
// When `__all__` is built dynamically the exports are unknown, so every name without a leading underscore might be
// public.
pub(crate) struct PublicApi<'a> {
    items: Vec<ApiItem<'a>>,
    // Each item's statement, and whether any item of it needs docs, so a lookup does not scan the items.
    statements: FxHashMap<TextRange, bool>,
    maybe_public: Vec<TextRange>,
    // Members of a public class whose docstring is shown when present but need none: dunders such as `__init__`, which
    // autodoc can render, overrides, which inherit the base docstring, and property setters.
    optional: FxHashSet<TextRange>,
    // The definitions the documentation names, such as an autodoc directive, which users read as public.
    documented: FxHashSet<TextRange>,
}

impl<'a> PublicApi<'a> {
    // Built once per file; rules read it through `Checker::public_api`.
    pub(crate) fn of(checker: &Checker<'a>) -> Self {
        let body = checker.body();
        let project = checker.project();
        let exports = checker.dunder_all();
        let reach = |name: &str| project.library_exposure(name);
        let dynamic = matches!(exports, Exports::Dynamic);
        let mut api = Self {
            items: Vec::new(),
            statements: FxHashMap::default(),
            maybe_public: Vec::new(),
            optional: FxHashSet::default(),
            documented: documented_definitions(checker),
        };
        // Definitions in top-level `if`, `try` and `match` blocks, as a version switch or an optional accelerator, are
        // the module's too.
        for (stmt, next) in module_statements_with_next(body) {
            match stmt {
                Stmt::FunctionDef(function) => {
                    let name = function.name.as_str();
                    match reach(name) {
                        // An `@overload` variant of a public function may carry a docstring, but needs none.
                        Some(_) if is_overload(&function.decorator_list) => {
                            api.optional.insert(function.range);
                        }
                        Some(exposure) => {
                            let range = function.name.range;
                            let kind = ApiKind::Function;
                            api.items.push(ApiItem {
                                stmt,
                                name: name.to_owned(),
                                range,
                                kind,
                                next: None,
                                via: exposure.reexporter(),
                                docs: exposure.is_offered(),
                            });
                        }
                        None if dynamic && Visibility::of(name) == Visibility::Public => {
                            api.maybe_public.push(function.range);
                        }
                        None => {}
                    }
                }
                Stmt::ClassDef(class) => match reach(class.name.as_str()) {
                    Some(exposure) => {
                        api.class(stmt, class, class.name.as_str(), exposure);
                        if !exposure.is_offered() {
                            api.inherited_members(class, &project);
                        }
                    }
                    None if dynamic && Visibility::of(&class.name) == Visibility::Public => {
                        api.maybe_public.push(class.range);
                    }
                    None => api.inherited_members(class, &project),
                },
                Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                    for target in targets.iter().filter_map(Expr::as_name_expr) {
                        if let Some(exposure) = reach(target.id.as_str()) {
                            api.variable(stmt, next, target, exposure);
                        }
                    }
                }
                Stmt::AnnAssign(ast::StmtAnnAssign { target: name, .. })
                | Stmt::TypeAlias(ast::StmtTypeAlias { name, .. }) => {
                    if let Some(target) = name.as_name_expr()
                        && let Some(exposure) = reach(target.id.as_str())
                    {
                        api.variable(stmt, next, target, exposure);
                    }
                }
                _ => {}
            }
        }
        for item in &api.items {
            *api.statements.entry(item.stmt.range()).or_default() |= item.docs;
        }
        api
    }

    fn variable(&mut self, stmt: &'a Stmt, next: Option<&'a Stmt>, target: &'a ast::ExprName, exposure: Exposure<'a>) {
        let name = target.id.to_string();
        let (via, docs) = (exposure.reexporter(), exposure.is_offered());
        self.items.push(ApiItem { stmt, name, range: target.range, kind: ApiKind::Variable, next, via, docs });
    }

    // The public methods and properties a class the file defines, such as a private mixin, that a public class of the
    // library inherits without overriding them: users reach them through that public class.
    fn inherited_members(&mut self, class: &'a ast::StmtClassDef, project: &FileIndex) {
        let name = class.name.as_str();
        for member in module_statements(&class.body) {
            let Stmt::FunctionDef(method) = member else {
                continue;
            };
            let Offer::Public(kind) = offer(method) else {
                continue;
            };
            let method_name = method.name.as_str();
            if !project.is_inherited_api(name, method_name) {
                continue;
            }
            // A method a signature-reached class defines is already an item, which its inheritance makes documented.
            if let Some(item) = self.items.iter_mut().find(|item| item.range == method.name.range) {
                item.docs = true;
                continue;
            }
            self.items.push(ApiItem {
                stmt: member,
                name: format!("{name}.{method_name}"),
                range: method.name.range,
                kind,
                next: None,
                via: None,
                docs: true,
            });
        }
    }

    // A public class, its public methods and properties, and its public nested classes with theirs.
    fn class(&mut self, stmt: &'a Stmt, class: &'a ast::StmtClassDef, name: &str, exposure: Exposure<'a>) {
        let range = class.name.range;
        let (via, docs) = (exposure.reexporter(), exposure.is_offered());
        self.items.push(ApiItem { stmt, name: name.to_owned(), range, kind: ApiKind::Class, next: None, via, docs });
        // Members in `if` and `try` blocks of the class body, as a version switch, are the class's too.
        for member in module_statements(&class.body) {
            match member {
                Stmt::ClassDef(nested) if Visibility::of(&nested.name) == Visibility::Public => {
                    self.class(member, nested, &format!("{name}.{}", nested.name), exposure);
                }
                Stmt::FunctionDef(method) => match offer(method) {
                    Offer::Optional => {
                        self.optional.insert(member.range());
                    }
                    Offer::Public(kind) => self.items.push(ApiItem {
                        stmt: member,
                        name: format!("{name}.{}", method.name),
                        range: method.name.range,
                        kind,
                        next: None,
                        via,
                        docs,
                    }),
                    Offer::Private => {}
                },
                _ => {}
            }
        }
    }

    // Whether the definition at `range` is part of the public API, as opposed to optional or possibly public: an item
    // of the API, or one the documentation names. Only the docstring rules need `Need::Docs`, which leaves out the items
    // only a signature reaches.
    pub(crate) fn is_public(&self, range: TextRange, need: Need) -> bool {
        self.is_item(range, need) || self.documented.contains(&range)
    }

    // Whether the definition at `range` may be part of the public API, which includes optional, documented and possibly
    // public ones. Only the docstring rules need `Need::Docs`, as for `is_public`.
    pub(crate) fn covers(&self, range: TextRange, need: Need) -> bool {
        self.is_item(range, need)
            || self.optional.contains(&range)
            || self.documented.contains(&range)
            || self.maybe_public.iter().any(|public| public.contains_range(range))
    }

    fn is_item(&self, range: TextRange, need: Need) -> bool {
        self.statements.get(&range).is_some_and(|docs| matches!(need, Need::Any) || *docs)
    }
}

// Which items count: the docstring rules require docs only for the names the library offers, while the other rules
// count every item, the names a signature reaches too.
#[derive(Clone, Copy)]
pub(crate) enum Need {
    Docs,
    Any,
}

// How a method of a public class is offered: dunders and overrides need no docstring, public methods and properties
// are documented, and the rest are private.
enum Offer {
    Optional,
    Public(ApiKind),
    Private,
}

fn offer(method: &ast::StmtFunctionDef) -> Offer {
    let decorators = decorator_names(&method.decorator_list);
    let visibility = Visibility::of(method.name.as_str());
    if visibility == Visibility::Dunder
        || decorators.iter().any(|decorator| matches!(*decorator, "override" | "setter" | "deleter" | "overload"))
    {
        Offer::Optional
    } else if visibility == Visibility::Public {
        Offer::Public(if decorators.contains(&"property") { ApiKind::Property } else { ApiKind::Method })
    } else {
        Offer::Private
    }
}

// The statements `module_statements` yields, each with the one after it in its own block, which may be inside an `if`,
// `try` or `match`.
fn module_statements_with_next(body: &[Stmt]) -> Vec<(&Stmt, Option<&Stmt>)> {
    fn collect<'a>(body: &'a [Stmt], found: &mut Vec<(&'a Stmt, Option<&'a Stmt>)>) {
        for (index, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                    collect(body, found);
                    for clause in elif_else_clauses {
                        collect(&clause.body, found);
                    }
                }
                Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                    collect(body, found);
                    for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                        collect(&handler.body, found);
                    }
                    collect(orelse, found);
                    collect(finalbody, found);
                }
                Stmt::Match(ast::StmtMatch { cases, .. }) => {
                    for case in cases {
                        collect(&case.body, found);
                    }
                }
                _ => found.push((stmt, body.get(index + 1))),
            }
        }
    }
    let mut found = Vec::new();
    collect(body, &mut found);
    found
}

// The module's functions, classes and class members that the documentation names by their dotted name under the module's
// package. A module's `__init__.py` names its package, so that segment is left out, as `ApiScope::role` does.
fn documented_definitions(checker: &Checker) -> FxHashSet<TextRange> {
    let stem = checker.file_name().strip_suffix(".py").unwrap_or(checker.file_name());
    let (module, anchored): (Vec<&str>, bool) = match checker.module_path() {
        Some(path) => (path.iter().map(String::as_str).collect(), true),
        None => (vec![stem], false),
    };
    let prefix =
        module.into_iter().filter(|part| *part != "__init__").fold(String::new(), |prefix, part| prefix + part + ".");
    let mut found = FxHashSet::default();
    documented_in(checker.body(), &prefix, anchored, &checker.project(), &mut found);
    found
}

fn documented_in(body: &[Stmt], prefix: &str, anchored: bool, project: &FileIndex, found: &mut FxHashSet<TextRange>) {
    for stmt in module_statements(body) {
        let (name, members) = match stmt {
            Stmt::FunctionDef(function) => (function.name.as_str(), &[][..]),
            Stmt::ClassDef(class) => (class.name.as_str(), class.body.as_slice()),
            _ => continue,
        };
        let qualified = format!("{prefix}{name}");
        if project.is_documented(&qualified, anchored) {
            found.insert(stmt.range());
        }
        documented_in(members, &format!("{qualified}."), anchored, project, found);
    }
}

// A variable is documented the two ways Sphinx autodoc reads: a string right after it, or `#:` comments right above.
fn is_documented_variable(checker: &Checker, stmt: &Stmt, next: Option<&Stmt>) -> bool {
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}

// Whether a comment after a docstring is a pragma or a tool directive. Such a comment covers the docstring's line only,
// so it goes with the docstring.
pub(crate) fn is_directive_comment(checker: &Checker, comment: TextRange) -> bool {
    let text = checker.text(comment);
    text.match_indices('#').any(|(start, _)| is_tool_directive(&text[start..]))
}

// Deletes the docstring that starts a body. A comment after it stays on a line of its own, unless it is a pragma or a
// tool directive, which goes with it.
pub(crate) fn docstring_deletion(checker: &Checker, body: &[Stmt]) -> Option<Edit> {
    let stmt = &body[0];
    let deletion = statement_deletion(checker, stmt.range(), body.get(1).map(Ranged::range), Blanks::Trailing)?;
    let comment = checker.trailing_comment(stmt).filter(|comment| !is_directive_comment(checker, *comment));
    match comment {
        Some(comment) if is_alone_on_lines(checker.source, stmt.range()) => {
            let newline = checker.line_ending();
            let line = format!("{}{}{newline}", indentation(checker.source, stmt), checker.text(comment));
            Some(Edit::range_replacement(line, deletion))
        }
        _ => Some(Edit::range_deletion(deletion)),
    }
}
