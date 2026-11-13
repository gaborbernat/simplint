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

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ModModule, Stmt};
use ruff_python_parser::{Parsed, parse_module};
use ruff_text_size::{TextRange, TextSize};

use self::api_reached::interface;
pub(crate) use self::api_reached::{Closure, Definition};
use self::module_reads::module_reads;
use self::runtime::{Runs, constants};
use self::scan::{Scan, string_words};
use self::statements::statements;
pub(crate) use self::statements::{Statement, statement_texts};
use self::visitor::{FactVisitor, grouped, top_level_bindings};
pub(crate) use self::visitor::{FromImport, ImportBinding, qualified};
use super::constructors::{CallContexts, Constructors};
use super::receivers::{AttributeWrites, module_name};
use crate::syntax::{Exports, dunder_all, module_statements};

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileFacts {
    pub(super) names: Channels<Vec<ModuleNames>>,
    pub(super) declared: Vec<String>,
    pub(super) assigned_attributes: Vec<String>,
    pub(super) module_writes: Vec<ModuleNames>,
    pub(super) module_reads: Vec<String>,
    pub(super) base_classes: Vec<String>,
    pub(super) star_imports: Vec<String>,
    pub(super) exports: Option<Vec<String>>,
    pub(super) exports_unknown: ExportsUnknown,
    pub(super) bindings: Vec<ImportBinding>,
    pub(super) module_names: Vec<(String, String)>,
    // A call through a module value reaches the module's names without an attribute read.
    pub(super) module_values: Vec<String>,
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
    // Parameters and imports request fixtures, and so does a string that spells a name, as `usefixtures("db")` does.
    pub(super) fixture_requests: Vec<String>,
    // The parameters of the file's `test*` functions and fixtures, which request fixtures wherever pytest collects the
    // file from, a module its `python_files` setting names as much as a test module.
    pub(super) pytest_requests: Vec<String>,
    pub(super) doctest_names: Vec<String>,
    pub(super) classes: Vec<ClassBases>,
    // An import of one of these names may bind a function rather than a class.
    pub(super) functions: Vec<String>,
    // The shape of each module-level function, which a call from another module checks its arguments against.
    pub(super) signatures: Vec<Signature>,
    pub(super) plain_modules: Vec<String>,
    pub(super) imported_modules: Vec<String>,
    pub(super) interface: Vec<Definition>,
    pub(super) runtime_imports: Vec<ImportSite>,
    pub(super) statements: Vec<Statement>,
    pub(super) parts: Parts,
    // A file holds thousands of words, which as one string the cache reads back in one allocation.
    pub(super) string_words: String,
    pub(super) global_names: Vec<String>,
    pub(super) bound_marks: Vec<String>,
    pub(super) source_digest: u64,
    // The search roots change the module paths a relative import resolves against, while the file stays the same.
    module_digest: u64,
}

impl FileFacts {
    // A cache file may hold facts that no longer match their source, when damaged or edited by hand.
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
        let constructors = constructors(path, (source, &parsed), &visitor, parts, &mut scan);
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
        Some(Self {
            names: visitor.names.map(grouped),
            declared,
            assigned_attributes: visitor.assigned_attributes,
            module_writes: grouped(visitor.module_writes),
            module_reads: reads,
            base_classes: visitor.base_classes,
            star_imports: visitor.stars.into_iter().collect(),
            exports,
            exports_unknown: match (dynamic_exports, module_getattr) {
                (true, _) => ExportsUnknown::Dynamic,
                (false, true) => ExportsUnknown::Lazy,
                (false, false) => ExportsUnknown::No,
            },
            bindings,
            module_values,
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
            signatures: if parts.has(Parts::SIGNATURES) { module_signatures(&parsed.syntax().body) } else { Vec::new() },
            plain_modules: visitor.plain_modules,
            imported_modules: visitor.imported_modules.into_iter().collect(),
            interface: interface(&parsed.syntax().body),
            fixture_requests: visitor.fixture_requests.into_iter().collect(),
            pytest_requests: visitor.pytest_requests.into_iter().collect(),
            doctest_names: visitor.doctest_names.into_iter().collect(),
            runtime_imports: visitor.runtime_sites,
            statements: if parts.has(Parts::STATEMENTS) { statements(&parsed, source) } else { Vec::new() },
            parts,
            string_words: string_words(scan.words),
            global_names: scan.global_names.into_iter().collect(),
            bound_marks: scan.bound_marks.into_iter().collect(),
            source_digest: source_digest(source),
            module_digest: module_digest(module_path, importable),
        })
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

// A function's name, whether calling it makes a coroutine (an `async def` that does not yield), the names of its
// positional parameters in order, and whether a decorator stands between its definition and what the name binds.
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
            coroutine: function.is_async && !crate::syntax::yields(&function.body),
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

// The facts only some rules read, each collected only for a run with one of those rules on, as collecting them costs
// a walk of the file or allocations for every name. Facts cached without a part a later run reads are collected again.
#[derive(Clone, Copy, Default, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Parts(u16);

impl CacheKey for Parts {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.0.cache_key(state);
    }
}

impl Parts {
    // The statements `duplicate-code` compares.
    pub const STATEMENTS: Self = Self(1);
    // The attributes read under computed names, which keep a definition `unused-definition` would report.
    pub const COMPUTED_READS: Self = Self(1 << 1);
    // Whether each call drops its value or runs the module as a program, which `test-only-parameter` and
    // `unused-return-value` tell production calls by.
    pub const CALL_CONTEXTS: Self = Self(1 << 2);
    // The names the file may request as fixtures, which `unused-fixture` reads.
    pub const FIXTURES: Self = Self(1 << 3);
    // The names doctest examples use, which hide a definition or a call from the rules that judge them by their uses.
    pub const DOCTESTS: Self = Self(1 << 4);
    // The classes the handlers catch, which `indistinct-error-class` reads.
    pub const CAUGHT: Self = Self(1 << 5);
    // The imports that run when the module loads, whose cycles `import-cycle` reports.
    pub const RUNTIME_IMPORTS: Self = Self(1 << 6);
    // The words of the file's string literals, which a rename of a module variable must leave to the strings naming it.
    pub const STRING_WORDS: Self = Self(1 << 8);
    // The names a `global` statement declares, which a function may rebind while the module runs.
    pub const GLOBAL_NAMES: Self = Self(1 << 9);
    // The names whose `__name__` or `__qualname__` the file reads, and whether it reads a class's name off an object,
    // which renaming a function or class changes.
    pub const REFLECTED_NAMES: Self = Self(1 << 10);
    // The modules a write under a computed name may set any name of, which a rename or move of their variables breaks.
    pub const COMPUTED_WRITES: Self = Self(1 << 11);
    // The modules the file imports, which `unused-dependency` matches to the project's requirements.
    pub const IMPORTED_MODULES: Self = Self(1 << 12);
    // The positional parameters of the module's functions, which the rules that judge a call by its callee read.
    pub const SIGNATURES: Self = Self(1 << 13);
    // The marks bound to a name, as `C_EXT = pytest.mark.c_extension`, which a conftest may register through the name,
    // so `unregistered-marker` leaves them alone.
    pub const BOUND_MARKS: Self = Self(1 << 14);

    #[must_use]
    pub fn with(self, part: Self) -> Self {
        Self(self.0 | part.0)
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
