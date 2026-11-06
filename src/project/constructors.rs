use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt};
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::computed_reads::ComputedReads;
use super::facts::{FromImport, qualified};
use super::index::FileId;
use crate::syntax::{attribute_call, is_main_guard};
use crate::syntax::{dataclass_decorator, keyword_is_true, module_statements};

// What `dataclass-kw-only` needs to rewrite every positional call of a dataclass it makes keyword-only: the dataclasses
// a file declares without `kw_only=True`, with the fields their `__init__` takes by position, and how the file uses
// names from other modules. `unused-default` reads the same uses for the calls of a function, and the methods a class
// defines with the attribute names the module loads.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Constructors {
    pub(super) dataclasses: Vec<(String, Option<Vec<String>>)>,
    pub(super) uses: Vec<(String, Vec<NameUse>)>,
    // The capitalized names called with positional arguments as attributes of something no import binds, such as
    // `module.Ranked(issue, score)` on a module a test loads from its file path; any dataclass so named may be the one.
    pub(super) dynamic: Vec<String>,
    // The methods of the module's top-level classes.
    pub(super) methods: Vec<String>,
    // The plain `name = value` attributes of the module's top-level classes, which `attribute_loads` counts like methods.
    pub(super) attributes: Vec<String>,
    // The module's own functions and classes, which an `obj.name` read reaches through the module as an object.
    pub(super) definitions: Vec<String>,
    // How many times the module loads each attribute name, wherever it appears, to count the `obj.name` reads of a
    // method name across the run.
    pub(super) attribute_loads: BTreeMap<String, u32>,
    // The attributes the module reads under names computed at run time.
    pub(super) computed_reads: ComputedReads,
}

// A name used from a module, as the file spells it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct NameUse {
    pub(super) name: String,
    pub(super) written: String,
    pub(super) kind: UseKind,
    // Whether the statement that makes the call drops its value, as `f()` on a line of its own does.
    pub(super) discarded: bool,
    // Whether the call runs the module as a program, from `__main__.py` or an `if __name__ == "__main__":` block.
    pub(super) entry: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) enum UseKind {
    // A call, by the arguments it passes.
    Call(CallShape),
    // A use no rewrite can follow: a call unpacking `*args`, a class pattern matching by position, or the name passed
    // around as a value.
    Opaque,
    // `from module import name as written`, or `from module import *` with `*` for the name.
    Imported,
}

// The arguments of a call that does not unpack `*args`: how many it passes by position, the names it passes by
// keyword, and whether it unpacks a mapping with `**`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct CallShape {
    pub(crate) positional: u32,
    pub(crate) keywords: Vec<String>,
    pub(crate) unpacks_mapping: bool,
}

impl CallShape {
    pub(crate) fn of(call: &ast::ExprCall) -> Self {
        let arguments = &call.arguments;
        Self {
            positional: u32::try_from(arguments.args.len()).unwrap_or(u32::MAX),
            keywords: arguments.keywords.iter().filter_map(|keyword| Some(keyword.arg.as_ref()?.to_string())).collect(),
            unpacks_mapping: arguments.keywords.iter().any(|keyword| keyword.arg.is_none()),
        }
    }
}

impl CacheKey for CallShape {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.positional.cache_key(state);
        self.keywords.cache_key(state);
        self.unpacks_mapping.cache_key(state);
    }
}

// Every call of a function the run makes, and whether some use of it is not a call, which hides the arguments it gets.
// The calls made from production code are kept apart, since the test-only rules compare them with the calls of tests.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct CallSites {
    pub(crate) opaque: bool,
    pub(crate) shapes: BTreeSet<CallShape>,
    pub(crate) production: BTreeSet<CallShape>,
    // Whether a production call uses the value it returns.
    pub(crate) production_reads: bool,
    // Whether code that runs the module as a program calls it, as an entry point would: its arguments and its value
    // belong to the command line, not to the run.
    pub(crate) entry: bool,
}

impl CallSites {
    pub(crate) fn add_shape(&mut self, shape: CallShape) {
        self.shapes.insert(shape);
    }

    pub(crate) fn add_call(&mut self, call: &ast::ExprCall) {
        match CallKind::of(call) {
            CallKind::Unpacked => self.opaque = true,
            _ => self.add_shape(CallShape::of(call)),
        }
    }

    // A call the defining module itself makes, which is production code.
    pub(crate) fn add_production_call(&mut self, call: &ast::ExprCall, discarded: bool, entry: bool) {
        if CallKind::of(call) == CallKind::Unpacked {
            self.opaque = true;
            return;
        }
        let shape = CallShape::of(call);
        self.add_production(&shape, !discarded, entry);
        self.shapes.insert(shape);
    }

    pub(crate) fn add_production(&mut self, shape: &CallShape, reads: bool, entry: bool) {
        if entry {
            self.entry = true;
            return;
        }
        self.production.insert(shape.clone());
        self.production_reads |= reads;
    }
}

impl CacheKey for CallSites {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.opaque.cache_key(state);
        self.shapes.iter().cloned().collect::<Vec<_>>().cache_key(state);
        self.production.iter().cloned().collect::<Vec<_>>().cache_key(state);
        self.production_reads.cache_key(state);
        self.entry.cache_key(state);
    }
}

impl Constructors {
    pub(super) fn collect(
        source: &str,
        tokens: &Tokens,
        body: &[Stmt],
        aliases: &FxHashMap<&str, String>,
        from_imports: &[FromImport],
        suppresses_rewrite: &dyn Fn(&str) -> bool,
        contexts: &CallContexts,
    ) -> Self {
        let qualify = |expr: &Expr| qualified(UnqualifiedName::from_expr(expr)?.segments(), aliases);
        let mut uses = Uses::new(|expr: &Expr| {
            let full = qualify(expr)?;
            let (module, member) = full.rsplit_once('.')?;
            Some((module.to_owned(), member.to_owned(), UnqualifiedName::from_expr(expr)?.segments().join(".")))
        });
        uses.visit_body(body);
        let mut dynamic = Uses::new(|expr: &Expr| {
            let ast::ExprAttribute { value, attr, .. } = expr.as_attribute_expr()?;
            let mut head = value.as_ref();
            while let Expr::Attribute(attribute) = head {
                head = &attribute.value;
            }
            let unbound = head.as_name_expr().is_none_or(|name| !aliases.contains_key(name.id.as_str()));
            (unbound && attr.starts_with(|first: char| first.is_ascii_uppercase())).then(|| attr.to_string())
        });
        dynamic.visit_body(body);
        let dynamic: BTreeSet<String> = dynamic
            .found
            .into_iter()
            .filter_map(|(name, found)| match found {
                Use::Call(call) if CallKind::of(call) != CallKind::Keyword => Some(name),
                _ => None,
            })
            .collect();
        let comments = OnceCell::new();
        let mut grouped: BTreeMap<String, BTreeSet<NameUse>> = BTreeMap::new();
        for ((module, name, written), found) in uses.found {
            let (kind, discarded, entry) = match found {
                Use::Call(call) => {
                    let (discarded, entry) = contexts.classify(call);
                    let kind = match CallKind::of(call) {
                        CallKind::Positional
                            if is_suppressed(
                                source,
                                comments.get_or_init(|| comment_ranges(tokens)),
                                call,
                                suppresses_rewrite,
                            ) =>
                        {
                            UseKind::Opaque
                        }
                        CallKind::Unpacked => UseKind::Opaque,
                        CallKind::Keyword | CallKind::Positional => UseKind::Call(CallShape::of(call)),
                    };
                    (kind, discarded, entry)
                }
                Use::Opaque => (UseKind::Opaque, false, false),
            };
            grouped.entry(module).or_default().insert(NameUse { name, written, kind, discarded, entry });
        }
        for FromImport { module, names, .. } in from_imports {
            for alias in *names {
                let written = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
                let name = alias.name.to_string();
                grouped.entry(module.clone()).or_default().insert(NameUse {
                    name,
                    written,
                    kind: UseKind::Imported,
                    discarded: false,
                    entry: false,
                });
            }
        }
        let is_dataclass = |expr: &Expr| qualify(expr).is_some_and(|name| name == "dataclasses.dataclass");
        let mut known: Vec<(&str, Option<Vec<String>>)> = Vec::new();
        let mut dataclasses: Vec<(String, Option<Vec<String>>)> = Vec::new();
        for class in body.iter().filter_map(Stmt::as_class_def_stmt) {
            let Some(decorator) = dataclass_decorator(class, &is_dataclass) else {
                continue;
            };
            let base_fields =
                |name: &str| known.iter().find(|(base, _)| *base == name).and_then(|(_, fields)| fields.clone());
            let fields = init_fields(class, decorator, &base_fields);
            if !keyword_is_true(decorator, "kw_only") {
                dataclasses.push((class.name.to_string(), fields.clone()));
            }
            known.push((class.name.as_str(), fields));
        }
        let Members { methods, attributes, definitions } = member_names(body);
        Self {
            dataclasses,
            uses: grouped.into_iter().map(|(module, uses)| (module, uses.into_iter().collect())).collect(),
            dynamic: dynamic.into_iter().collect(),
            methods,
            attributes,
            definitions,
            attribute_loads: attribute_loads(body),
            computed_reads: ComputedReads::default(),
        }
    }
}

// Where the module's calls sit: those whose value a statement drops, and the blocks that run the module as a program.
// Only the rules that compare production calls read them, so a run without those rules leaves them empty.
#[derive(Default)]
pub(crate) struct CallContexts {
    discarded: FxHashSet<TextRange>,
    entry: Vec<TextRange>,
}

impl CallContexts {
    pub(crate) fn of(body: &[Stmt], is_main: bool) -> Self {
        Self { discarded: discarded_calls(body), entry: entry_blocks(body, is_main) }
    }

    // Whether the call's value is dropped, and whether the call sits in code that runs the module as a program.
    pub(crate) fn classify(&self, call: &ast::ExprCall) -> (bool, bool) {
        let range = call.range();
        (self.discarded.contains(&range), self.entry.iter().any(|block| block.contains_range(range)))
    }
}

// The names a module's members bind: the methods and plain `name = value` attributes of its top-level classes, and its
// own functions and classes.
struct Members {
    methods: Vec<String>,
    attributes: Vec<String>,
    definitions: Vec<String>,
}

// A member in an `if` or `try` block of the class body is the class's too, as `module_statements` reads it.
fn member_names(body: &[Stmt]) -> Members {
    let classes = body.iter().filter_map(Stmt::as_class_def_stmt);
    let methods = classes
        .clone()
        .flat_map(|class| module_statements(&class.body).filter_map(Stmt::as_function_def_stmt))
        .map(|function| function.name.to_string())
        .collect();
    let attributes = classes
        .flat_map(|class| module_statements(&class.body).filter_map(Stmt::as_assign_stmt))
        .flat_map(|assign| assign.targets.iter().filter_map(Expr::as_name_expr))
        .map(|target| target.id.to_string())
        .collect();
    let definitions = module_statements(body)
        .filter_map(|stmt| match stmt {
            Stmt::FunctionDef(function) => Some(function.name.to_string()),
            Stmt::ClassDef(class) => Some(class.name.to_string()),
            _ => None,
        })
        .collect();
    Members { methods, attributes, definitions }
}

// The calls that make up a whole statement, whose value nothing reads, such as `f()` on a line of its own.
fn discarded_calls(body: &[Stmt]) -> FxHashSet<TextRange> {
    struct Discarded(FxHashSet<TextRange>);
    impl<'a> Visitor<'a> for Discarded {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt
                && let Expr::Call(call) = value.as_ref()
            {
                self.0.insert(call.range());
            }
            visitor::walk_stmt(self, stmt);
        }
    }
    let mut discarded = Discarded(FxHashSet::default());
    discarded.visit_body(body);
    discarded.0
}

// The code that runs only when the module runs as a program: the whole of a `__main__.py`, or the module's
// `if __name__ == "__main__":` blocks.
fn entry_blocks(body: &[Stmt], is_main: bool) -> Vec<TextRange> {
    if is_main {
        return body.iter().map(Ranged::range).collect();
    }
    body.iter().filter(|stmt| is_main_guard(stmt)).map(Ranged::range).collect()
}

// A suppression comment on the call keeps its own `dataclass-kw-only` report from rewriting it.
fn is_suppressed(
    source: &str,
    comments: &CommentRanges,
    call: &ast::ExprCall,
    suppresses_rewrite: &dyn Fn(&str) -> bool,
) -> bool {
    comments
        .comments_in_range(source.full_lines_range(call.range()))
        .iter()
        .any(|comment| suppresses_rewrite(&source[*comment]))
}

// The attribute names the body reads, with how often: `obj.name`, `obj.name(...)` and `getattr(obj, "name")` alike.
fn attribute_loads(body: &[Stmt]) -> BTreeMap<String, u32> {
    let mut reads = crate::syntax::ExprCollector::new(|expr: &Expr| {
        if let Some(ast::ExprAttribute { attr, ctx, .. }) = expr.as_attribute_expr() {
            return matches!(ctx, ast::ExprContext::Load).then(|| attr.to_string());
        }
        // Unlike `ModuleReads`, `hasattr` is no load: it only probes for the name.
        let (_, name) = attribute_call(expr, &["getattr"])?;
        Some(name.as_string_literal_expr()?.value.to_str().to_owned())
    });
    reads.visit_body(body);
    let mut loads = BTreeMap::new();
    for name in reads.found {
        *loads.entry(name).or_default() += 1;
    }
    loads
}

fn comment_ranges(tokens: &Tokens) -> CommentRanges {
    CommentRanges::new(tokens.iter().filter(|token| token.kind() == TokenKind::Comment).map(Ranged::range).collect())
}

// Over the run: how other files use each file's dataclasses, and for each file the fields of the dataclasses it calls
// with positional arguments, by how it spells the call.
// Over the run: how other files use each file's dataclasses, the calls of each file's functions, and for each file the
// fields of the dataclasses it calls with positional arguments, by how it spells the call.
#[derive(Default)]
pub struct RunCalls {
    // Per file, its dataclasses another file uses in a way no rewrite follows, which includes importing one from a
    // module that passes it on to others.
    opaque: FxHashMap<FileId, FxHashSet<String>>,
    star_imported: FxHashSet<FileId>,
    fields: FxHashMap<FileId, FxHashMap<String, Vec<String>>>,
    // Per file, the calls of each function it defines, from the other files of the run.
    functions: FxHashMap<FileId, FxHashMap<String, CallSites>>,
    // Per file, how many times the run loads each of the file's method names.
    method_loads: FxHashMap<FileId, FxHashMap<String, u32>>,
    // The modules only tests import, directly or through other such modules, whose calls are test code.
    test_support: FxHashSet<FileId>,
}

type Declared<'f> = &'f [(String, Option<Vec<String>>)];

// A file, whether its own reports rewrite its positional calls, whether it is production code rather than a test module,
// and the files its names resolve to with the names it uses from each.
pub(super) struct FileUses<'f> {
    pub(super) caller: FileId,
    pub(super) rewrites_calls: bool,
    pub(super) production: bool,
    pub(super) modules: Vec<(FileId, &'f [NameUse])>,
}

impl RunCalls {
    // `defined` holds the names each file binds at its top level, which tell a re-export from a definition. `hidden`
    // holds the names some file may read off a module under a computed name, so any function so named may be called.
    pub(super) fn index<'f>(
        declared: impl Iterator<Item = (FileId, Declared<'f>)>,
        uses: &[FileUses],
        imported: &FxHashMap<FileId, FxHashSet<String>>,
        dynamic: &FxHashSet<&str>,
        defined: &FxHashMap<FileId, FxHashSet<&str>>,
        hidden: &FxHashSet<&str>,
    ) -> Self {
        let declared: FxHashMap<FileId, Declared<'f>> = declared.filter(|(_, classes)| !classes.is_empty()).collect();
        let mut calls = Self::default();
        for (file, classes) in &declared {
            let called = classes.iter().filter(|(class, _)| dynamic.contains(class.as_str()));
            for (class, _) in called {
                calls.opaque.entry(*file).or_default().insert(class.clone());
            }
        }
        let mut reexported: FxHashSet<String> = FxHashSet::default();
        for FileUses { caller, rewrites_calls, production, modules } in uses {
            for (file, used) in modules {
                for NameUse { name, kind, discarded, entry, .. } in *used {
                    let sites = calls.functions.entry(*file).or_default().entry(name.clone()).or_default();
                    match kind {
                        UseKind::Call(shape) => {
                            sites.add_shape(shape.clone());
                            if *production {
                                sites.add_production(shape, !discarded, *entry);
                            }
                        }
                        UseKind::Opaque => sites.opaque = true,
                        UseKind::Imported => {}
                    }
                    if !defined.get(file).is_some_and(|names| names.contains(name.as_str())) {
                        reexported.insert(name.clone());
                    }
                }
                let Some(classes) = declared.get(file) else {
                    continue;
                };
                for NameUse { name, written, kind, .. } in *used {
                    if *kind == UseKind::Imported && name == "*" {
                        calls.star_imported.insert(*file);
                        continue;
                    }
                    let Some((_, fields)) = classes.iter().find(|(class, _)| class == name) else {
                        continue;
                    };
                    let opaque = match (kind, fields) {
                        (UseKind::Imported, _) => imported.get(caller).is_some_and(|names| names.contains(written)),
                        (UseKind::Call(shape), _) if shape.positional == 0 => false,
                        (UseKind::Call(shape), Some(fields))
                            if *rewrites_calls && shape.positional as usize <= fields.len() =>
                        {
                            calls.fields.entry(*caller).or_default().insert(written.clone(), fields.clone());
                            false
                        }
                        _ => true,
                    };
                    if opaque {
                        calls.opaque.entry(*file).or_default().insert(name.clone());
                    }
                }
            }
        }
        // A function another module reaches through a name this file does not define, as `pkg.helper` through a
        // package that re-exports it, may be called under any file's name for it.
        for (file, names) in defined {
            for name in names.iter().filter(|name| reexported.contains(**name) || hidden.contains(**name)) {
                calls.mark_opaque(*file, (*name).to_owned());
            }
        }
        calls
    }

    // Records that some use of a function the file defines hides its arguments, as an entry point or a dotted string
    // naming it does.
    pub(super) fn mark_opaque(&mut self, file: FileId, name: String) {
        self.functions.entry(file).or_default().entry(name).or_default().opaque = true;
    }

    pub(super) fn set_method_loads(&mut self, file: FileId, loads: FxHashMap<String, u32>) {
        self.method_loads.insert(file, loads);
    }

    pub(super) fn file(&self, id: Option<FileId>) -> FileCalls<'_> {
        FileCalls {
            opaque: id.and_then(|id| self.opaque.get(&id)),
            star_imported: id.is_some_and(|id| self.star_imported.contains(&id)),
            fields: id.and_then(|id| self.fields.get(&id)),
            functions: id.and_then(|id| self.functions.get(&id)),
            method_loads: id.and_then(|id| self.method_loads.get(&id)),
            test_support: id.is_some_and(|id| self.test_support.contains(&id)),
        }
    }

    pub(super) fn set_test_support(&mut self, files: FxHashSet<FileId>) {
        self.test_support = files;
    }
}

// What the index knows of one file's calls: how other files use its dataclasses and functions, the dataclasses of
// other files it calls by position, and how often the run loads its method names.
#[derive(Clone, Copy, PartialEq)]
pub struct FileCalls<'a> {
    opaque: Option<&'a FxHashSet<String>>,
    star_imported: bool,
    fields: Option<&'a FxHashMap<String, Vec<String>>>,
    functions: Option<&'a FxHashMap<String, CallSites>>,
    method_loads: Option<&'a FxHashMap<String, u32>>,
    test_support: bool,
}

impl<'a> FileCalls<'a> {
    // Whether only tests import the module, directly or through other modules only tests import, so its calls serve
    // them.
    pub(crate) fn is_test_support(&self) -> bool {
        self.test_support
    }

    // A star import hides which names the importing file uses, so it could use any dataclass in any way.
    pub fn has_opaque_use(&self, name: &str) -> bool {
        self.star_imported || self.opaque.is_some_and(|names| names.contains(name))
    }

    pub fn fields(&self, written: &str) -> Option<&'a [String]> {
        self.fields?.get(written).map(Vec::as_slice)
    }

    pub fn calls_any(&self) -> bool {
        self.fields.is_some()
    }

    // The calls the other files make of the function `name` this file defines, or `None` when they make none.
    pub(crate) fn call_sites(&self, name: &str) -> Option<&'a CallSites> {
        self.functions?.get(name)
    }

    // How many times the run loads the attribute `name`, which counts every `obj.name` read of a method of that name.
    pub(crate) fn attribute_loads(&self, name: &str) -> u32 {
        self.method_loads.and_then(|loads| loads.get(name)).copied().unwrap_or_default()
    }
}

impl CacheKey for FileCalls<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.opaque.cache_key(state);
        self.star_imported.cache_key(state);
        self.fields.cache_key(state);
        self.functions.cache_key(state);
        self.method_loads.cache_key(state);
        self.test_support.cache_key(state);
    }
}

// The fields the generated `__init__` takes by position, in order; `None` when simplint cannot tell them, as when the
// class writes its own `__init__` or inherits from a class it does not know. A base dataclass of the same module,
// whose fields `base_fields` gives, comes first, as dataclasses order inherited fields.
pub(crate) fn init_fields(
    class: &ast::StmtClassDef,
    decorator: &Expr,
    base_fields: &impl Fn(&str) -> Option<Vec<String>>,
) -> Option<Vec<String>> {
    if decorator
        .as_call_expr()
        .and_then(|call| call.arguments.find_keyword("init"))
        .is_some_and(|init| !init.value.as_boolean_literal_expr().is_some_and(|literal| literal.value))
        || class.body.iter().any(|stmt| {
            stmt.as_function_def_stmt().is_some_and(|function| matches!(function.name.as_str(), "__init__" | "__new__"))
        })
        || class.keywords().iter().any(|keyword| keyword.arg.is_none())
    {
        return None;
    }
    let mut fields: Vec<String> = Vec::new();
    for base in class.bases() {
        match UnqualifiedName::from_expr(subscripted(base)).as_ref().map(UnqualifiedName::segments) {
            Some([.., "Generic" | "ABC" | "object" | "Protocol"]) => {}
            Some([name]) if fields.is_empty() => fields = base_fields(name)?,
            _ => return None,
        }
    }
    let keyword_only = keyword_is_true(decorator, "kw_only");
    let mut rest_keyword_only = false;
    let declared = class.body.iter().filter_map(|stmt| {
        let field = stmt.as_ann_assign_stmt()?;
        Some((field.target.as_name_expr()?.id.as_str(), field))
    });
    for (name, ast::StmtAnnAssign { annotation, value, .. }) in declared {
        if let Some(text) = annotation.as_string_literal_expr().map(|literal| literal.value.to_str()) {
            if text.contains("ClassVar") || text.contains("KW_ONLY") {
                return None;
            }
        } else {
            match UnqualifiedName::from_expr(subscripted(annotation))
                .as_ref()
                .and_then(|name| name.segments().last().copied())
            {
                Some("ClassVar") => continue,
                Some("KW_ONLY") => {
                    rest_keyword_only = true;
                    continue;
                }
                _ => {}
            }
        }
        let options = value.as_deref().and_then(|value| match value {
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if UnqualifiedName::from_expr(func).is_some_and(|func| func.segments().last() == Some(&"field")) =>
            {
                Some(arguments)
            }
            _ => None,
        });
        let flag = |option: &str, default: bool| match options.and_then(|arguments| arguments.find_keyword(option)) {
            None => Some(default),
            Some(keyword) => keyword.value.as_boolean_literal_expr().map(|literal| literal.value),
        };
        if options.is_some_and(|arguments| arguments.keywords.iter().any(|keyword| keyword.arg.is_none())) {
            return None;
        }
        let in_init = flag("init", true)?;
        let by_keyword = flag("kw_only", keyword_only || rest_keyword_only)?;
        if in_init && !by_keyword && !fields.iter().any(|field| field == name) {
            fields.push(name.to_owned());
        }
    }
    Some(fields)
}

fn subscripted(expr: &Expr) -> &Expr {
    expr.as_subscript_expr().map_or(expr, |subscript| &subscript.value)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallKind {
    Keyword,
    Positional,
    Unpacked,
}

impl CallKind {
    // `**kwargs` passes by keyword, which `kw_only=True` leaves working; only `*args` can carry a field by position.
    pub(crate) fn of(call: &ast::ExprCall) -> Self {
        let arguments = &call.arguments;
        if arguments.args.iter().any(Expr::is_starred_expr) {
            Self::Unpacked
        } else if arguments.args.is_empty() {
            Self::Keyword
        } else {
            Self::Positional
        }
    }
}

pub(crate) enum Use<'a> {
    Call(&'a ast::ExprCall),
    Opaque,
}

// Every use of the names `matches` recognizes, keyed by what it returns. Annotations, `isinstance` and `issubclass`
// checks, base classes, and attribute reads such as `Name.method` neither call the class nor hand it to code that
// might, so they are not uses.
pub(crate) struct Uses<'a, K, F> {
    matches: F,
    pub(crate) found: Vec<(K, Use<'a>)>,
}

impl<'a, K, F: Fn(&Expr) -> Option<K>> Uses<'a, K, F> {
    pub(crate) fn new(matches: F) -> Self {
        Self { matches, found: Vec::new() }
    }

    fn skip_or_visit(&mut self, expr: &'a Expr) {
        if (self.matches)(expr).is_none() {
            self.visit_expr(expr);
        }
    }
}

impl<'a, K, F: Fn(&Expr) -> Option<K>> Visitor<'a> for Uses<'a, K, F> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let Stmt::ClassDef(class) = stmt else {
            return visitor::walk_stmt(self, stmt);
        };
        for decorator in &class.decorator_list {
            self.visit_decorator(decorator);
        }
        for base in class.bases() {
            self.skip_or_visit(base);
        }
        for keyword in class.keywords() {
            self.visit_keyword(keyword);
        }
        self.visit_body(&class.body);
    }

    fn visit_annotation(&mut self, _: &'a Expr) {}

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(key) = (self.matches)(expr) {
            self.found.push((key, Use::Opaque));
            return;
        }
        match expr {
            Expr::Call(call) if let Some(key) = (self.matches)(&call.func) => {
                self.found.push((key, Use::Call(call)));
                self.visit_arguments(&call.arguments);
            }
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if let [subject, classes] = &*arguments.args
                    && func
                        .as_name_expr()
                        .is_some_and(|name| matches!(name.id.as_str(), "isinstance" | "issubclass")) =>
            {
                self.visit_expr(subject);
                match classes {
                    Expr::Tuple(tuple) => tuple.iter().for_each(|class| self.skip_or_visit(class)),
                    other => self.skip_or_visit(other),
                }
            }
            Expr::Attribute(ast::ExprAttribute { value, .. }) => self.skip_or_visit(value),
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        if let Pattern::MatchClass(ast::PatternMatchClass { cls, arguments, .. }) = pattern
            && let Some(key) = (self.matches)(cls)
        {
            if !arguments.patterns.is_empty() {
                self.found.push((key, Use::Opaque));
            }
            return self.visit_pattern_arguments(arguments);
        }
        visitor::walk_pattern(self, pattern);
    }
}
