mod api_reached;
mod module_reads;
mod names;
mod runtime;
mod scan;
mod statements;
mod visitor;

pub(crate) use visitor::doctest_code;

use crate::project::computed_reads::ComputedReads;
use std::collections::BTreeSet;
use std::hash::Hasher;
use std::mem::take;
use std::path::Path;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ModModule, Stmt};
use ruff_python_parser::{Parsed, parse_module};
use ruff_text_size::{TextRange, TextSize};

use self::api_reached::interface;
pub(crate) use self::api_reached::{Closure, Definition};
use self::module_reads::module_reads;
use self::names::string_words;
use self::runtime::{Runs, falsy_constants};
use self::scan::Scan;
use self::statements::statements;
pub(crate) use self::statements::{Statement, statement_texts};
use self::visitor::{FactVisitor, grouped, top_level_bindings};
pub(crate) use self::visitor::{FromImport, ImportBinding, qualified};
use super::constructors::{CallContexts, Constructors};
use super::receivers::{AttributeWrites, module_name};
use crate::syntax::{Exports, dunder_all, module_statements};

// Why the names a module provides may not all be known: `__all__` is built in a way parsing cannot follow, or a
// module-level `__getattr__` supplies names lazily, on top of the names the module defines.
#[derive(Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) enum ExportsUnknown {
    No,
    Dynamic,
    Lazy,
}

// The names a file uses from other modules, before they resolve to files; parsing finds them and the cache keeps them.
// Names are grouped by module, so each module resolves once per file.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileFacts {
    pub(super) imports: Vec<ModuleNames>,
    pub(super) references: Vec<ModuleNames>,
    // The modules and names a docstring's examples read, which a call-site rule must see but which patch nothing.
    pub(super) doctest_references: Vec<ModuleNames>,
    pub(super) declared: Vec<String>,
    pub(super) assigned_attributes: Vec<String>,
    // The names a `<receiver>.setattr` call sets on each module, as `monkeypatch.setattr(module, "NAME", value)`.
    pub(super) module_writes: Vec<ModuleNames>,
    // The attributes read off module objects whose module parsing cannot tell, any of which may be a name of this run.
    pub(super) module_reads: Vec<String>,
    pub(super) base_classes: Vec<String>,
    // The modules the file star-imports, which hand it whatever names their `import *` exports.
    pub(super) star_imports: Vec<String>,
    // The names a static `__all__` lists, which are all `import *` takes; without one it takes every public name.
    pub(super) exports: Option<Vec<String>>,
    // Whether the names the module provides are known, and why not.
    pub(super) exports_unknown: ExportsUnknown,
    // The names the module's own top-level statements import from other modules, which `__all__`, a redundant alias
    // such as `from .core import run as run`, or the absence of `__all__` may re-export.
    pub(super) bindings: Vec<ImportBinding>,
    // The dotted module each name bound at the top level stands for, such as `pkg.findings` for `from . import findings`
    // in `pkg`, which resolves to a file only when the name is a module.
    pub(super) module_names: Vec<(String, String)>,
    // The names the module's top-level statements define, which a module without `__all__` exports.
    pub(super) defined: Vec<String>,
    pub(super) attribute_writes: AttributeWrites,
    // The modules a write under a computed name reaches, as `pkg.report` for `setattr(report, name, value)` or
    // `mock.patch(f"pkg.report.{name}")`, whose names any of them may be.
    pub(super) computed_writes: Vec<String>,
    pub(super) constructors: Constructors,
    // Whether the file reads a docstring off an object, through `__doc__` or a `getdoc` call.
    pub(super) reads_docstrings: bool,
    // Whether the file reads a class's name off an object, as `type(obj).__name__` does, so renaming a class changes
    // it.
    pub(super) reads_class_names: bool,
    // The names whose `__name__` or `__qualname__` the file reads, as `helper` for `helper.__name__` or `Item` for
    // `pkg.Item.__qualname__`, which renaming the function or class changes.
    pub(super) reflected_names: Vec<String>,
    // The classes handlers catch, and those `isinstance` checks and class patterns name inside a handler, as
    // (module, name).
    pub(super) caught: Vec<(String, String)>,
    // The names the file may request as fixtures: its function parameters, the names it imports, those whose `__name__`
    // it reads, and any string that spells a name, as `usefixtures("db")` does. The index keeps those of test modules,
    // where pytest reads them.
    pub(super) fixture_requests: Vec<String>,
    // The parameters of the file's `test*` functions and fixtures, which request fixtures wherever pytest collects the
    // file from, a module its `python_files` setting names as much as a test module.
    pub(super) pytest_requests: Vec<String>,
    // The names the file's doctest examples, the `>>>` lines of its strings, use, which run code no import shows.
    pub(super) doctest_names: Vec<String>,
    // The module-level classes, each with the classes it lists as bases, as (module, name).
    pub(super) classes: Vec<ClassBases>,
    // The names of the module-level functions, which an import of the run may bind as a function rather than a class.
    pub(super) functions: Vec<String>,
    // The modules the top-level `import pkg.module` statements bind by their root name, which a module's attributes reach.
    pub(super) plain_modules: Vec<(String, String)>,
    // The module's top-level definitions with the names their interfaces refer to, which the index follows from the names
    // the module exposes to find the classes and aliases a signature reaches.
    pub(super) interface: Vec<Definition>,
    // The import statements that run when the module loads, each with the module it imports.
    pub(super) runtime_imports: Vec<ImportSite>,
    // The statements of every body that `duplicate-code` compares across files, each body's in a row.
    pub(super) statements: Vec<Statement>,
    // The facts above only some rules read that were collected.
    pub(super) parts: Parts,
    // The words of the file's string literals, which a string in any file of the run may spell a name by, one per line:
    // a file holds thousands, which as one string the cache reads back in one allocation.
    pub(super) string_words: String,
    // The names a `global` statement declares, which a function rebinds at run time.
    pub(super) global_names: Vec<String>,
    // A digest of the source the facts come from, whose offsets the ranges above are.
    pub(super) source_digest: u64,
    // A digest of the module paths the facts resolved relative imports and module names against, which change with
    // the search roots while the file stays the same.
    module_digest: u64,
}

// A module an import statement names, the `module.name` of each name a `from` import takes from it, the statement's
// range in the file, and the `(major, minor)` Python version it runs only below, if a version check guards it.
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

impl FileFacts {
    // Whether every range of the facts lies in a source of `length` bytes, and the statements of each body follow one
    // another, which facts a cache file holds may not when it was damaged or edited by hand.
    pub fn fits(&self, length: u64) -> bool {
        let sites = self.runtime_imports.iter().map(|site| (site.start, site.end));
        let statements = self.statements.iter().map(|statement| (statement.start, statement.end));
        sites.chain(statements).all(|(start, end)| start <= end && u64::from(end) <= length)
            && self.statements.windows(2).all(|pair| pair[0].body != pair[1].body || pair[0].end <= pair[1].start)
    }

    // The module path, such as `["package", "module"]`, is what relative imports resolve against. A file outside any
    // package has none, and resolves them against `importable`, the module path under its search root, if it has one.
    pub fn collect(
        path: &Path,
        source: &str,
        module_path: Option<&[String]>,
        importable: Option<&[String]>,
        parts: Parts,
    ) -> Option<Self> {
        let parsed = parse_module(source).ok()?;
        let mut imports = FactVisitor::new(
            path,
            module_path.or(importable),
            module_name(module_path.or(importable), path),
            parts,
            falsy_constants(&parsed.syntax().body),
        );
        imports.visit_body(&parsed.syntax().body);
        let functions = module_functions(&parsed.syntax().body);
        let classes = module_statements(&parsed.syntax().body)
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
            .collect();
        // A stub publishes what it declares, so its names are the API of the module it describes.
        let declared = if path.extension().is_some_and(|extension| extension == "pyi") {
            parsed.syntax().body.iter().flat_map(declared_names).collect()
        } else {
            Vec::new()
        };
        let attribute_writes =
            AttributeWrites::collect(&parsed.syntax().body, &imports.aliases, &module_name(module_path, path));
        let mut scan = Scan::of(&parsed.syntax().body, parts);
        let constructors = constructors(path, (source, &parsed), &imports, parts, &mut scan);
        // A module whose spec no static path resolves reads names of any module, as a computed import does.
        let mut modules = scan.modules;
        modules.extend(imports.unknown_modules.iter().cloned());
        let reads = module_reads(&parsed.syntax().body, &modules)
            .into_iter()
            .chain(imports.unresolved_globals.iter().cloned())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let listed = dunder_all(&parsed.syntax().body);
        let interface = interface(&parsed.syntax().body);
        let (exports, dynamic_exports) = match listed {
            Exports::Listed(names) => (Some(names.into_iter().map(str::to_owned).collect()), false),
            Exports::Absent => (None, false),
            Exports::Dynamic => (None, true),
        };
        let (mut defined, mut module_getattr) = (Vec::new(), false);
        for stmt in module_statements(&parsed.syntax().body) {
            // PEP 562's module `__getattr__` answers for names the module does not bind, as lazily loaded submodules.
            module_getattr |= stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__getattr__");
            defined.extend(declared_names(stmt));
        }
        let bindings = top_level_bindings(&imports.from_statements);
        let module_names = bindings
            .iter()
            .map(|binding| (binding.bound.clone(), format!("{}.{}", binding.module, binding.name)))
            .chain(imports.module_aliases)
            .collect();
        Some(Self {
            imports: grouped(imports.edges),
            references: grouped(imports.references),
            doctest_references: grouped(imports.doctest_references),
            declared,
            assigned_attributes: imports.assigned_attributes,
            module_writes: grouped(imports.module_writes),
            module_reads: reads,
            base_classes: imports.base_classes,
            star_imports: imports.stars.into_iter().collect(),
            exports,
            exports_unknown: match (dynamic_exports, module_getattr) {
                (true, _) => ExportsUnknown::Dynamic,
                (false, true) => ExportsUnknown::Lazy,
                (false, false) => ExportsUnknown::No,
            },
            bindings,
            module_names,
            defined,
            attribute_writes,
            computed_writes: imports.computed_writes.into_iter().collect(),
            constructors,
            reads_docstrings: imports.reads_docstrings,
            reads_class_names: imports.reads_class_names,
            reflected_names: imports.reflected_names.into_iter().collect(),
            caught: imports.caught,
            classes,
            functions,
            plain_modules: imports.plain_modules,
            interface,
            fixture_requests: imports.fixture_requests.into_iter().collect(),
            pytest_requests: imports.pytest_requests.into_iter().collect(),
            doctest_names: imports.doctest_names.into_iter().collect(),
            runtime_imports: imports.runtime_sites,
            statements: if parts.has(Parts::STATEMENTS) { statements(&parsed, source) } else { Vec::new() },
            parts,
            string_words: string_words(scan.words),
            global_names: scan.global_names.into_iter().collect(),
            source_digest: source_digest(source),
            module_digest: module_digest(module_path, importable),
        })
    }

    // Whether the facts hold every part a run's rules read, resolved against the module paths the file has in the run.
    pub fn covers(&self, wanted: Parts, module_path: Option<&[String]>, importable: Option<&[String]>) -> bool {
        self.parts.has(wanted) && self.module_digest == module_digest(module_path, importable)
    }

    pub fn parts(&self) -> Parts {
        self.parts
    }
}

fn module_digest(module_path: Option<&[String]>, importable: Option<&[String]>) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    module_path.cache_key(&mut hasher);
    importable.cache_key(&mut hasher);
    hasher.finish()
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

    #[must_use]
    pub fn with(self, part: Self) -> Self {
        Self(self.0 | part.0)
    }

    pub fn has(self, part: Self) -> bool {
        self.0 & part.0 == part.0
    }
}

pub(super) fn source_digest(source: &str) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    source.cache_key(&mut hasher);
    hasher.finish()
}

// The calls and definitions of the module that dataclass and call rules read, with the parts only some rules read when
// `parts` asks for them.
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
    let mut constructors =
        Constructors::collect(source, parsed.tokens(), body, &imports.aliases, &imports.from_statements, &contexts);
    constructors.attribute_loads = take(&mut scan.attribute_loads);
    constructors.computed_reads = take(&mut scan.computed_reads);
    constructors
}

// A module-level class, the classes it lists as bases, each as (module, name), and the names of the methods it defines.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ClassBases {
    pub(super) name: String,
    pub(super) bases: Vec<(String, String)>,
    pub(super) methods: Vec<String>,
}

// A module and the names used from it.
pub(super) type ModuleNames = (String, Vec<String>);

impl FileFacts {
    // The names the file imports from modules or reads off them, attributes of objects included, which a project outside
    // the file's own may use.
    pub fn imported_names(&self) -> impl Iterator<Item = &str> {
        self.imports.iter().flat_map(|(_, names)| names.iter().map(String::as_str))
    }

    // The attributes the file stores to or deletes, which a project outside the file's own may store to as well.
    pub fn assigned_names(&self) -> impl Iterator<Item = &str> {
        self.assigned_attributes.iter().map(String::as_str)
    }

    // The names doctest examples of the file use.
    pub fn doctests(&self) -> impl Iterator<Item = &str> {
        self.doctest_names.iter().map(String::as_str)
    }

    // The attributes the file reads under computed names.
    pub(crate) fn computed_reads(&self) -> &ComputedReads {
        &self.constructors.computed_reads
    }

    pub fn read_names(&self) -> impl Iterator<Item = &str> {
        self.imports
            .iter()
            .chain(&self.references)
            .chain(&self.doctest_references)
            .flat_map(|(_, names)| names.iter().map(String::as_str))
            .chain(self.module_reads.iter().map(String::as_str))
            .chain(self.constructors.attribute_loads.keys().map(String::as_str))
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
