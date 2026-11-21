mod api_reached;
mod caught;
mod doctests;
mod fixtures;
mod loaded;
mod module_reads;
mod patches;
mod runtime;
mod scan;
mod statements;
mod visitor;

pub(crate) use doctests::doctest_code;
pub(in crate::project) use fixtures::PYTEST_PLUGINS_REQUEST;

use crate::project::computed_reads::ComputedReads;
use crate::syntax::is_main_guard;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hasher;
use std::mem::take;
use std::ops::{Index, IndexMut};
use std::path::Path;

use itertools::Itertools;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ModModule, Stmt};
use ruff_python_parser::{Parsed, parse_module};
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::FxHashMap;

use self::api_reached::interface;
pub(crate) use self::api_reached::{Closure, Definition};
use self::module_reads::{DynamicReads, dynamic_modules, module_reads};
use self::runtime::{Runs, constants};
use self::scan::{Scan, string_words};
use self::statements::statements;
pub(crate) use self::statements::{Statement, statement_texts};
use self::visitor::{FactVisitor, grouped, top_level_bindings};
pub(crate) use self::visitor::{FromImport, ImportBinding, qualified};
use super::constructors::{CallContexts, Constructors};
use super::receivers::{AttributeWrites, module_name};
use crate::syntax::{Exports, dunder_all, module_statements, yields};

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileFacts {
    pub(super) names: Channels<Vec<ModuleNames>>,
    pub(super) declared: Vec<String>,
    pub(super) assigned_attributes: Vec<String>,
    pub(super) module_writes: Vec<ModuleNames>,
    pub(super) module_reads: Vec<String>,
    pub(super) base_classes: Vec<String>,
    pub(super) mixins: Vec<String>,
    pub(super) star_imports: Vec<String>,
    pub(super) exports: Option<Vec<String>>,
    pub(super) conditional_exports: Vec<String>,
    pub(super) exports_unknown: ExportsUnknown,
    pub(super) bindings: Vec<ImportBinding>,
    pub(super) module_names: Vec<(String, String)>,
    // A call through a module value reaches the module's names without an attribute read.
    pub(super) module_values: Vec<String>,
    pub(super) dynamic: DynamicReads,
    pub(super) main_guard: bool,
    pub(super) defined: Vec<String>,
    pub(super) attribute_writes: AttributeWrites,
    pub(super) computed_writes: Vec<String>,
    pub(super) constructors: Constructors,
    pub(super) attribute_loads: BTreeMap<String, u32>,
    pub(super) computed_reads: ComputedReads,
    pub(super) reads_docstrings: bool,
    pub(super) reads_class_names: bool,
    pub(super) reflected_names: Vec<String>,
    pub(super) caught: Vec<(String, String)>,
    pub(super) class_tuples: Vec<(String, Vec<(String, String)>)>,
    // A string such as "db" can request a fixture through usefixtures or getfixturevalue.
    pub(super) fixture_requests: Vec<String>,
    // Tests and fixtures can request these parameters even in files outside the recognized test paths.
    pub(super) pytest_requests: Vec<String>,
    pub(super) doctest_names: Vec<String>,
    pub(super) classes: Vec<ClassBases>,
    // An import of one of these names may bind a function rather than a class.
    pub(super) functions: Vec<String>,
    pub(super) signatures: Vec<Signature>,
    pub(super) plain_modules: Vec<String>,
    pub(super) imported_modules: Vec<String>,
    pub(super) interface: Vec<Definition>,
    pub(super) runtime_imports: Vec<ImportSite>,
    pub(super) statements: Vec<Statement>,
    pub(super) parts: Parts,
    // Store words together so the cache can read them back in one allocation.
    pub(super) string_words: String,
    pub(super) global_names: Vec<String>,
    pub(super) bound_marks: Vec<String>,
    pub(super) source_digest: u64,
    // The search roots change the module paths a relative import resolves against, while the file stays the same.
    module_digest: u64,
}

impl FileFacts {
    // Check cached source offsets before using them. A damaged archive can contain out-of-range values.
    pub fn fits(&self, length: u64) -> bool {
        let sites = self.runtime_imports.iter().map(|site| (site.start, site.end));
        let statements = self.statements.iter().map(|statement| (statement.start, statement.end));
        sites.chain(statements).all(|(start, end)| start <= end && u64::from(end) <= length)
            && self.statements.windows(2).all(|pair| pair[0].body != pair[1].body || pair[0].end <= pair[1].start)
    }

    pub fn collect(
        path: &Path,
        source: &str,
        module_path: Option<&[String]>,
        importable: Option<&[String]>,
        parts: Parts,
    ) -> Option<Self> {
        let parsed = parse_module(source).ok()?;
        Some(Self::from_parse(path, source, module_path, importable, parts, &parsed))
    }

    pub(crate) fn from_parse(
        path: &Path,
        source: &str,
        module_path: Option<&[String]>,
        importable: Option<&[String]>,
        parts: Parts,
        parsed: &Parsed<ModModule>,
    ) -> Self {
        // A file outside any package has no module path, so its relative imports resolve against its importable path.
        let mut visitor = FactVisitor::new(
            path,
            module_path.or(importable),
            module_name(module_path.or(importable), path),
            parts,
            constants(&parsed.syntax().body),
        );
        visitor.visit_body(&parsed.syntax().body);
        visitor.finish_module();
        let classes = module_classes(&parsed.syntax().body, &visitor);
        // A stub publishes what it declares, so its names are the API of the module it describes.
        let declared = if path.extension().is_some_and(|extension| extension == "pyi") {
            parsed.syntax().body.iter().flat_map(declared_names).collect()
        } else {
            Vec::new()
        };
        let attribute_writes =
            AttributeWrites::collect(&parsed.syntax().body, &visitor.aliases, &module_name(module_path, path));
        let mut scan = Scan::of(&parsed.syntax().body, parts);
        let constructors = constructors(path, (source, parsed), &visitor, parts, &mut scan);
        // A module whose spec no static path resolves reads names of any module, as a computed import does.
        let mut modules = scan.modules;
        modules.extend(visitor.unknown_modules.iter().cloned());
        let reads = module_reads(&parsed.syntax().body, &modules)
            .into_iter()
            .chain(visitor.unresolved_globals.iter().cloned())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let (exports, dynamic_exports) = match dunder_all(&parsed.syntax().body) {
            Exports::Listed(names) => (Some(names.names().iter().copied().map(str::to_owned).collect()), false),
            Exports::Absent => (None, false),
            Exports::Dynamic => (None, true),
        };
        let (mut defined, mut module_getattr) = (Vec::new(), false);
        for stmt in module_statements(&parsed.syntax().body) {
            // PEP 562's module `__getattr__` answers for names the module does not bind, as lazily loaded submodules.
            module_getattr |= stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__getattr__");
            defined.extend(declared_names(stmt));
        }
        let bindings = top_level_bindings(&visitor.from_statements);
        let (module_names, module_values) = module_bindings(&bindings, &visitor);
        let bound: FxHashMap<&str, &str> =
            module_names.iter().map(|(bound, module)| (bound.as_str(), module.as_str())).collect();
        let dynamic = dynamic_modules(&parsed.syntax().body, |expr| match expr {
            Expr::Name(name) => bound.get(name.id.as_str()).map(|module| (*module).to_owned()),
            other => visitor.qualified(other),
        });
        Self {
            names: visitor.names.map(grouped),
            declared,
            assigned_attributes: visitor.assigned_attributes,
            module_writes: grouped(visitor.module_writes),
            module_reads: reads,
            base_classes: visitor.base_classes,
            mixins: visitor.mixins,
            star_imports: visitor.stars.into_iter().collect(),
            exports,
            conditional_exports: conditional_package_exports(path, &parsed.syntax().body, dynamic_exports),
            exports_unknown: match (dynamic_exports, module_getattr) {
                (true, _) => ExportsUnknown::Dynamic,
                (false, true) => ExportsUnknown::Lazy,
                (false, false) => ExportsUnknown::No,
            },
            bindings,
            module_values,
            dynamic,
            main_guard: parsed.syntax().body.iter().any(is_main_guard),
            module_names,
            defined,
            attribute_writes,
            computed_writes: visitor.computed_writes.into_iter().collect(),
            constructors,
            attribute_loads: take(&mut scan.attribute_loads),
            computed_reads: take(&mut scan.computed_reads),
            reads_docstrings: visitor.reads_docstrings,
            reads_class_names: visitor.reads_class_names,
            reflected_names: visitor.reflected_names.into_iter().collect(),
            caught: visitor.caught,
            class_tuples: visitor.class_tuples,
            classes,
            functions: module_functions(&parsed.syntax().body),
            signatures: if parts.has(Parts::SIGNATURES) {
                module_signatures(&parsed.syntax().body)
            } else {
                Vec::new()
            },
            plain_modules: visitor.plain_modules,
            imported_modules: visitor.imported_modules.into_iter().collect(),
            interface: interface(&parsed.syntax().body),
            fixture_requests: visitor.fixture_requests.into_iter().collect(),
            pytest_requests: visitor.pytest_requests.into_iter().collect(),
            doctest_names: visitor.doctest_names.into_iter().collect(),
            runtime_imports: visitor.runtime_sites,
            statements: if parts.has(Parts::STATEMENTS) { statements(parsed, source) } else { Vec::new() },
            parts,
            string_words: string_words(scan.words),
            global_names: scan.global_names.into_iter().collect(),
            bound_marks: scan.bound_marks.into_iter().collect(),
            source_digest: source_digest(source),
            module_digest: module_digest(module_path, importable),
        }
    }

    pub fn covers(&self, wanted: Parts, module_path: Option<&[String]>, importable: Option<&[String]>) -> bool {
        self.parts.has(wanted) && self.module_digest == module_digest(module_path, importable)
    }

    pub fn parts(&self) -> Parts {
        self.parts
    }

    pub fn imported_names(&self) -> impl Iterator<Item = &str> {
        self.names[Channel::Import].iter().flat_map(|(_, names)| names.iter().map(String::as_str))
    }

    pub fn assigned_names(&self) -> impl Iterator<Item = &str> {
        self.assigned_attributes.iter().map(String::as_str)
    }

    pub fn doctests(&self) -> impl Iterator<Item = &str> {
        self.doctest_names.iter().map(String::as_str)
    }

    pub(crate) fn computed_reads(&self) -> &ComputedReads {
        &self.computed_reads
    }

    pub fn read_names(&self) -> impl Iterator<Item = &str> {
        self.names
            .iter()
            .flatten()
            .flat_map(|(_, names)| names.iter().map(String::as_str))
            .chain(self.module_reads.iter().map(String::as_str))
            .chain(self.attribute_loads.keys().map(String::as_str))
    }
}

fn conditional_package_exports(path: &Path, body: &[Stmt], dynamic: bool) -> Vec<String> {
    if dynamic && (path.ends_with("__init__.py") || path.ends_with("__init__.pyi")) {
        return conditional_exports(body).unwrap_or_default();
    }
    Vec::new()
}

// A package can offer different names on different platforms. Other rules need the list for a single execution.
fn conditional_exports(body: &[Stmt]) -> Option<Vec<String>> {
    let writers = body.iter().filter(|stmt| !matches!(dunder_all(std::slice::from_ref(stmt)), Exports::Absent));
    let branch = writers.exactly_one().ok()?.as_if_stmt()?;
    let mut names = Vec::new();
    for body in std::iter::once(&branch.body).chain(branch.elif_else_clauses.iter().map(|clause| &clause.body)) {
        match dunder_all(body) {
            Exports::Absent => {}
            Exports::Listed(listed) => names.extend(listed.names().iter().copied().map(str::to_owned)),
            Exports::Dynamic => names.extend(conditional_exports(body)?),
        }
    }
    Some(names)
}

#[derive(Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) enum ExportsUnknown {
    No,
    Dynamic,
    Lazy,
}

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ImportSite {
    pub(super) module: String,
    pub(super) submodules: Vec<String>,
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) below: Option<(u8, u8)>,
}

impl ImportSite {
    fn new(module: String, submodules: Vec<String>, range: TextRange, runs: Runs) -> Self {
        let below = match runs {
            Runs::Below(major, minor) => Some((major, minor)),
            Runs::Always | Runs::Never => None,
        };
        Self { module, submodules, start: range.start().to_u32(), end: range.end().to_u32(), below }
    }

    pub(super) fn range(&self) -> TextRange {
        TextRange::new(TextSize::new(self.start), TextSize::new(self.end))
    }
}

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ClassBases {
    pub(super) name: String,
    pub(super) bases: Vec<(String, String)>,
    pub(super) methods: Vec<String>,
}

fn module_classes(body: &[Stmt], imports: &FactVisitor<'_>) -> Vec<ClassBases> {
    module_statements(body)
        .filter_map(Stmt::as_class_def_stmt)
        .map(|class| ClassBases {
            name: class.name.to_string(),
            bases: class.bases().iter().filter_map(|base| imports.class_path(base)).collect(),
            methods: class
                .body
                .iter()
                .filter_map(Stmt::as_function_def_stmt)
                .map(|function| function.name.to_string())
                .collect(),
        })
        .collect()
}

// Store the signature details callers need without keeping the syntax tree.
#[derive(Clone, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Signature {
    pub name: String,
    pub coroutine: bool,
    pub positional: Vec<String>,
    pub decorated: bool,
}

impl ruff_cache::CacheKey for Signature {
    fn cache_key(&self, state: &mut ruff_cache::CacheKeyHasher) {
        self.name.cache_key(state);
        self.coroutine.cache_key(state);
        self.positional.cache_key(state);
        self.decorated.cache_key(state);
    }
}

impl Signature {
    pub fn of(function: &ast::StmtFunctionDef) -> Self {
        let parameters = &function.parameters;
        Self {
            name: function.name.to_string(),
            coroutine: function.is_async && !yields(&function.body),
            positional: parameters
                .posonlyargs
                .iter()
                .chain(&parameters.args)
                .map(|parameter| parameter.name().to_string())
                .collect(),
            decorated: !function.decorator_list.is_empty(),
        }
    }
}

fn module_signatures(body: &[Stmt]) -> Vec<Signature> {
    module_statements(body).filter_map(Stmt::as_function_def_stmt).map(Signature::of).collect()
}

fn module_functions(body: &[Stmt]) -> Vec<String> {
    module_statements(body).filter_map(Stmt::as_function_def_stmt).map(|function| function.name.to_string()).collect()
}

fn declared_names(stmt: &Stmt) -> Vec<String> {
    match stmt {
        Stmt::FunctionDef(function) => vec![function.name.to_string()],
        Stmt::ClassDef(class) => vec![class.name.to_string()],
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.to_string()).collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })
        | Stmt::TypeAlias(ast::StmtTypeAlias { name: target, .. }) => {
            target.as_name_expr().map(|name| name.id.to_string()).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

fn constructors(
    path: &Path,
    (source, parsed): (&str, &Parsed<ModModule>),
    imports: &FactVisitor,
    parts: Parts,
    scan: &mut Scan<'_>,
) -> Constructors {
    let body = &parsed.syntax().body;
    let contexts = if parts.has(Parts::CALL_CONTEXTS) {
        CallContexts::new(take(&mut scan.discarded), body, path.file_name().is_some_and(|name| name == "__main__.py"))
    } else {
        CallContexts::default()
    };
    Constructors::collect(source, parsed.tokens(), body, &imports.aliases, &imports.from_statements, &contexts)
}

fn module_bindings(bindings: &[ImportBinding], imports: &FactVisitor<'_>) -> (Vec<(String, String)>, Vec<String>) {
    let module_names: Vec<(String, String)> = bindings
        .iter()
        .map(|binding| (binding.bound.clone(), format!("{}.{}", binding.module, binding.name)))
        .chain(imports.module_aliases.iter().cloned())
        .collect();
    let values = module_values(&module_names, imports);
    (module_names, values)
}

fn module_values(module_names: &[(String, String)], imports: &FactVisitor<'_>) -> Vec<String> {
    let dotted = imports
        .dotted_loads
        .iter()
        .filter_map(|dotted| visitor::qualified(&dotted.split('.').collect::<Vec<_>>(), &imports.aliases));
    module_names
        .iter()
        .filter(|(bound, _)| imports.value_loads.contains(bound.as_str()))
        .map(|(_, module)| module.clone())
        .chain(dotted)
        .chain(imports.nested_modules.iter().cloned())
        .collect()
}

fn module_digest(module_path: Option<&[String]>, importable: Option<&[String]>) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    module_path.cache_key(&mut hasher);
    importable.cache_key(&mut hasher);
    hasher.finish()
}

pub(super) fn source_digest(source: &str) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    source.cache_key(&mut hasher);
    hasher.finish()
}

// Some rules need another tree walk or extra allocations. Reuse cached facts only if they include every part requested
// this time.
#[derive(Clone, Copy, Default, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Parts(u16);

impl CacheKey for Parts {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.0.cache_key(state);
    }
}

impl Parts {
    pub const STATEMENTS: Self = Self(1);
    pub const COMPUTED_READS: Self = Self(1 << 1);
    // Production-call checks need to distinguish command-line calls and discarded results.
    pub const CALL_CONTEXTS: Self = Self(1 << 2);
    pub const FIXTURES: Self = Self(1 << 3);
    // Doctests may use a definition or call it outside the indexed Python statements.
    pub const DOCTESTS: Self = Self(1 << 4);
    pub const CAUGHT: Self = Self(1 << 5);
    pub const RUNTIME_IMPORTS: Self = Self(1 << 6);
    // A string can refer to a module variable by name even when no Python expression reads it.
    pub const STRING_WORDS: Self = Self(1 << 8);
    pub const GLOBAL_NAMES: Self = Self(1 << 9);
    // A rename changes the value of __name__ and __qualname__.
    pub const REFLECTED_NAMES: Self = Self(1 << 10);
    // A write with a computed name can still target a renamed or moved variable.
    pub const COMPUTED_WRITES: Self = Self(1 << 11);
    pub const IMPORTED_MODULES: Self = Self(1 << 12);
    pub const SIGNATURES: Self = Self(1 << 13);
    // A conftest can register a mark indirectly through an alias such as `C_EXT = pytest.mark.c_extension`.
    pub const BOUND_MARKS: Self = Self(1 << 14);

    #[must_use]
    pub fn with(self, part: Self) -> Self {
        Self(self.0 | part.0)
    }

    // Duplicate-code excludes test modules, so their statements never enter a comparison.
    #[must_use]
    pub fn for_file(self, is_test: bool) -> Self {
        if is_test { Self(self.0 & !Self::STATEMENTS.0) } else { self }
    }

    pub fn has(self, part: Self) -> bool {
        self.0 & part.0 == part.0
    }
}

// Grouped by module, so each module resolves once per file.
pub(super) type ModuleNames = (String, Vec<String>);

#[derive(Clone, Copy)]
pub(super) enum Channel {
    Import,
    Reference,
    Doctest,
}

#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Channels<T>(pub(super) [T; 3]);

impl<T> Channels<T> {
    pub(super) fn map<U>(self, to: impl FnMut(T) -> U) -> Channels<U> {
        Channels(self.0.map(to))
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
}

impl<T> Index<Channel> for Channels<T> {
    type Output = T;

    fn index(&self, channel: Channel) -> &T {
        &self.0[channel as usize]
    }
}

impl<T> IndexMut<Channel> for Channels<T> {
    fn index_mut(&mut self, channel: Channel) -> &mut T {
        &mut self.0[channel as usize]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use test_case::test_case;

    use super::{FileFacts, Parts};

    const SOURCE: &str = "import os\n\nif True:\n    VALUE = 1\n    OTHER = 2\n";

    fn collected() -> FileFacts {
        let parts = Parts::default().with(Parts::RUNTIME_IMPORTS).with(Parts::STATEMENTS);
        FileFacts::collect(Path::new("module.py"), SOURCE, None, None, parts).expect("facts")
    }

    #[test_case(&|_| {}, true ; "as_collected")]
    #[test_case(&|facts| facts.runtime_imports[0].end = 50, false ; "import_past_the_source")]
    #[test_case(&|facts| facts.runtime_imports[0].start = 20, false ; "import_ending_before_it_starts")]
    #[test_case(&|facts| facts.statements[1].start = 99, false ; "statement_ending_before_it_starts")]
    #[test_case(&|facts| facts.statements.swap(1, 2), false ; "statements_out_of_order")]
    fn facts_fit_the_source_they_came_from(damage: &dyn Fn(&mut FileFacts), fits: bool) {
        let mut facts = collected();
        damage(&mut facts);
        assert_eq!(facts.fits(SOURCE.len() as u64), fits);
    }
}
