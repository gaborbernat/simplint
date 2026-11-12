use ruff_macros::CacheKey;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use rustc_hash::{FxHashMap, FxHashSet};

// The class docstring Sphinx renders, from `class-doc-from` or `autoclass_content`.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, CacheKey, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub enum AutoclassContent {
    // The class docstring alone, which Sphinx renders when no option sets another.
    #[default]
    Class,
    // The `__init__` docstring alone, or the class docstring when `__init__` has none.
    Init,
    Both,
    // A value the rules cannot read, so which docstring Sphinx renders is unknown and no copy is removed.
    Unknown,
}

impl AutoclassContent {
    pub(super) fn from_option(value: &str) -> Self {
        match value {
            "class" => Self::Class,
            "init" => Self::Init,
            "both" => Self::Both,
            _ => Self::Unknown,
        }
    }
}

// The docstring of a project's classes, when its Sphinx configurations agree. Sphinx reads each configuration of
// a project on its own, so one that sets nothing renders the class docstring, and two that disagree leave it unknown.
pub(super) fn agreed(values: impl IntoIterator<Item = AutoclassContent>) -> AutoclassContent {
    let distinct: FxHashSet<AutoclassContent> = values.into_iter().collect();
    match distinct.len() {
        0 => AutoclassContent::Class,
        1 => distinct.into_iter().next().unwrap_or_default(),
        _ => AutoclassContent::Unknown,
    }
}

// The docstring a Sphinx configuration renders for its classes, or `None` when the file is not a Sphinx configuration.
// Sphinx's master document is `index`, and a configuration names its extensions or master document, so one with neither
// does not count. A file that does not parse may still be one, so its docstring is unknown.
pub(super) fn class_doc_from(text: &str, has_index: bool) -> Option<AutoclassContent> {
    let Ok(parsed) = parse_module(text) else {
        return Some(AutoclassContent::Unknown);
    };
    let scan = Scan::of(&parsed.syntax().body);
    if !has_index && !scan.bound("extensions") && !scan.bound("master_doc") && !scan.bound("root_doc") {
        return None;
    }
    Some(scan.class_doc_from())
}

const OPTION: &str = "autoclass_content";
const DEFAULT_OPTIONS: &str = "autodoc_default_options";
const TRACKED: [&str; 5] = [OPTION, DEFAULT_OPTIONS, "extensions", "master_doc", "root_doc"];

fn tracked(name: &str) -> Option<&'static str> {
    TRACKED.iter().copied().find(|tracked| *tracked == name)
}

fn literal(value: &Expr) -> Option<&str> {
    value.as_string_literal_expr().map(|literal| literal.value.to_str())
}

fn is_name(func: &Expr, names: [&str; 4]) -> bool {
    func.as_name_expr().is_some_and(|name| names.contains(&name.id.as_str()))
}

// The names a configuration binds that the rules read. A top-level assignment of the option in a form read statically
// is recorded; any other way of binding one of the names, nested in a block, through an import, a star import, `exec`,
// or a mutation such as `globals()[...]`, makes its value unknown, since Sphinx may then render either docstring.
#[derive(Default)]
struct Scan<'a> {
    bound: FxHashSet<&'static str>,
    unknown: FxHashSet<&'static str>,
    values: FxHashMap<&'static str, &'a Expr>,
}

impl<'a> Scan<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut scan = Self::default();
        for stmt in body {
            if !scan.assigned(stmt) {
                scan.visit_stmt(stmt);
            }
        }
        scan
    }

    fn bound(&self, name: &str) -> bool {
        tracked(name).is_some_and(|name| self.bound.contains(name))
    }

    // Records a top-level assignment of a tracked name, or a tuple assignment, whose values are read statically. The
    // values and any subscript targets are still scanned, so `exec` or a mutation inside them is seen.
    fn assigned(&mut self, stmt: &'a Stmt) -> bool {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match (targets.as_slice(), value.as_ref()) {
                ([Expr::Name(name)], value) if tracked(name.id.as_str()).is_some() => {
                    self.record(name.id.as_str(), value);
                    self.visit_expr(value);
                    true
                }
                ([Expr::Tuple(targets)], Expr::Tuple(values))
                    if targets.elts.len() == values.elts.len()
                        && !targets.elts.iter().chain(&values.elts).any(|expr| matches!(expr, Expr::Starred(_))) =>
                {
                    for (target, value) in targets.elts.iter().zip(&values.elts) {
                        match target.as_name_expr() {
                            Some(name) if tracked(name.id.as_str()).is_some() => self.record(name.id.as_str(), value),
                            _ => self.visit_expr(target),
                        }
                        self.visit_expr(value);
                    }
                    true
                }
                _ => false,
            },
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. })
                if target.as_name_expr().is_some_and(|name| tracked(name.id.as_str()).is_some()) =>
            {
                if let Some(name) = target.as_name_expr() {
                    self.record(name.id.as_str(), value);
                }
                self.visit_expr(value);
                true
            }
            _ => false,
        }
    }

    fn record(&mut self, name: &str, value: &'a Expr) {
        if let Some(name) = tracked(name) {
            self.bound.insert(name);
            self.values.insert(name, value);
        }
    }

    fn bind(&mut self, name: &str) {
        if let Some(name) = tracked(name) {
            self.bound.insert(name);
            if name == OPTION || name == DEFAULT_OPTIONS {
                self.unknown.insert(name);
            }
        }
    }

    fn bind_all(&mut self) {
        self.bound.extend(TRACKED);
        self.unknown.extend([OPTION, DEFAULT_OPTIONS]);
    }

    // A change to what a name holds, such as a key set in a dict, is a change the scan cannot read.
    fn mutate(&mut self, value: &Expr) {
        if let Some(name) = value.as_name_expr() {
            self.bind(name.id.as_str());
        }
    }

    fn class_doc_from(&self) -> AutoclassContent {
        self.default_options().or_else(|| self.option()).unwrap_or_default()
    }

    // The option's value at the top level, which `autodoc_default_options` overrides for every class.
    fn option(&self) -> Option<AutoclassContent> {
        if self.unknown.contains(OPTION) {
            return Some(AutoclassContent::Unknown);
        }
        self.values
            .get(OPTION)
            .map(|value| literal(value).map_or(AutoclassContent::Unknown, AutoclassContent::from_option))
    }

    // The `class-doc-from` key of the dict literal, the last one the dict sets, when it sets one.
    fn default_options(&self) -> Option<AutoclassContent> {
        if self.unknown.contains(DEFAULT_OPTIONS) {
            return Some(AutoclassContent::Unknown);
        }
        let Expr::Dict(dict) = self.values.get(DEFAULT_OPTIONS)? else {
            return Some(AutoclassContent::Unknown);
        };
        let mut found = None;
        for item in &dict.items {
            match item.key.as_ref().map(literal) {
                Some(Some("class-doc-from")) => {
                    found = Some(literal(&item.value).map_or(AutoclassContent::Unknown, AutoclassContent::from_option));
                }
                Some(Some(_)) => {}
                None | Some(None) => return Some(AutoclassContent::Unknown),
            }
        }
        found
    }
}

impl<'a> Visitor<'a> for Scan<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                    self.bind(bound.split('.').next().unwrap_or(bound));
                }
            }
            Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    if alias.name.as_str() == "*" {
                        self.bind_all();
                    } else {
                        self.bind(alias.asname.as_ref().unwrap_or(&alias.name).as_str());
                    }
                }
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
                self.bind(name.as_str());
            }
            Stmt::Try(ast::StmtTry { handlers, .. }) => {
                for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                    if let Some(name) = &handler.name {
                        self.bind(name.as_str());
                    }
                }
            }
            Stmt::Match(_) => self.bind_all(),
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) if matches!(name.ctx, ast::ExprContext::Store | ast::ExprContext::Del) => {
                self.bind(name.id.as_str());
            }
            Expr::Attribute(ast::ExprAttribute { value, .. })
            | Expr::Subscript(ast::ExprSubscript {
                value, ctx: ast::ExprContext::Store | ast::ExprContext::Del, ..
            }) => {
                self.mutate(value);
            }
            Expr::Call(ast::ExprCall { func, .. }) if is_name(func, ["exec", "globals", "locals", "vars"]) => {
                self.bind_all();
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
