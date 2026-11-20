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
    #[default]
    Class,
    // Sphinx tries the `__init__` docstring, then `__new__`, then the class docstring.
    Init,
    Both,
    // Keep both copies when the configuration is ambiguous.
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

// An explicit autoclass or autoexception directive can inherit `class-doc-from` from `autodoc_default_options`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rendered {
    pub(super) default: AutoclassContent,
    pub(super) named: AutoclassContent,
}

impl Rendered {
    pub(super) const UNKNOWN: Self = Self { default: AutoclassContent::Unknown, named: AutoclassContent::Unknown };
}

// Several documentation builds may describe the same project. Conflicting choices make a docstring removal unsafe.
pub(super) fn agreed(values: impl IntoIterator<Item = AutoclassContent>) -> AutoclassContent {
    let distinct: FxHashSet<AutoclassContent> = values.into_iter().collect();
    match distinct.len() {
        0 => AutoclassContent::Class,
        1 => distinct.into_iter().next().unwrap_or_default(),
        _ => AutoclassContent::Unknown,
    }
}

// Recognize Sphinx by a neighboring index or by its extension or root-document settings. A parse error leaves the
// settings unknown.
pub(super) fn class_doc_from(text: &str, has_index: bool) -> Option<Rendered> {
    let Ok(parsed) = parse_module(text) else {
        return Some(Rendered::UNKNOWN);
    };
    let scan = Scan::of(&parsed.syntax().body);
    if !has_index && !scan.bound("extensions") && !scan.bound("master_doc") && !scan.bound("root_doc") {
        return None;
    }
    Some(scan.rendered())
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

// Sphinx executes conf.py. The scan reads simple top-level assignments without running the file.
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

    // An assignment can change other settings through a call in its value or in a subscript target.
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
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => match target.as_ref() {
                Expr::Name(name) if tracked(name.id.as_str()).is_some() => {
                    self.record(name.id.as_str(), value);
                    self.visit_expr(value);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn record(&mut self, name: &str, value: &'a Expr) {
        let name = tracked(name).expect("only a tracked name is recorded");
        self.bound.insert(name);
        self.values.insert(name, value);
    }

    fn bind(&mut self, name: &str) {
        if let Some(name) = tracked(name) {
            self.bound.insert(name);
            if name == OPTION || name == DEFAULT_OPTIONS {
                self.unknown.insert(name);
            }
        }
    }

    // A wildcard import can override either option without identifying the file as a Sphinx configuration.
    fn bind_all(&mut self) {
        self.unknown.extend([OPTION, DEFAULT_OPTIONS]);
    }

    // Mutating an option invalidates its literal value.
    fn mutate(&mut self, value: &Expr) {
        if let Some(name) = value.as_name_expr() {
            self.bind(name.id.as_str());
        }
    }

    fn rendered(&self) -> Rendered {
        Rendered {
            default: self.option().unwrap_or_default(),
            named: self.default_options().or_else(|| self.option()).unwrap_or_default(),
        }
    }

    fn option(&self) -> Option<AutoclassContent> {
        if self.unknown.contains(OPTION) {
            return Some(AutoclassContent::Unknown);
        }
        self.values
            .get(OPTION)
            .map(|value| literal(value).map_or(AutoclassContent::Unknown, AutoclassContent::from_option))
    }

    // A later literal key overrides an earlier one. An unpacked mapping or computed key leaves the value unknown.
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
            Expr::Call(ast::ExprCall { func, .. })
                if func
                    .as_name_expr()
                    .is_some_and(|name| matches!(name.id.as_str(), "exec" | "globals" | "locals" | "vars")) =>
            {
                self.bind_all();
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoclassContent, Rendered, class_doc_from};

    #[test]
    fn imported_extensions_mark_a_configuration() {
        let class = Rendered { default: AutoclassContent::Class, named: AutoclassContent::Class };
        assert_eq!(class_doc_from("from shared import extensions\n", false), Some(class));
    }
}
