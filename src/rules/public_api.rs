use std::path::Path;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::project::{Exposure, FileIndex};
use crate::syntax::{Exports, Visibility, decorator_names, is_overload, module_statements};

pub(super) enum ApiKind {
    Function,
    Class,
    Method,
    Property,
    Variable,
}

pub(super) struct ApiItem<'a> {
    pub(super) stmt: &'a Stmt,
    pub(super) name: String,
    pub(super) range: TextRange,
    pub(super) kind: ApiKind,
    // The statement after a variable's in its block, where its docstring goes.
    pub(super) next: Option<&'a Stmt>,
    // The file that re-exports the item to users, when its own module does not offer it.
    pub(super) via: Option<&'a Path>,
    // Whether the docstring rules require docs for the item: not when only a signature of an exposed name reaches it.
    pub(super) docs: bool,
}

// The public API of a library is `FileIndex::library_exposure`, the names its users can import, with the public members
// of those classes. An application has none. What the docstring rules add is documentation: autodoc-documented
// definitions, whose names only the checker's module path makes qualified, and the members that need no docstring.
// When `__all__` is built dynamically the exports are unknown, so every name without a leading underscore might be
// public.
pub(crate) struct PublicApi<'a> {
    pub(super) items: Vec<ApiItem<'a>>,
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
pub(super) fn module_statements_with_next(body: &[Stmt]) -> Vec<(&Stmt, Option<&Stmt>)> {
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
