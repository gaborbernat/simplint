use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt};
use ruff_python_parser::parse_string_annotation;
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use super::facts::{FromImport, qualified};
use super::index::FileId;
use crate::syntax::is_main_guard;
use crate::syntax::{
    ExprCollector, alias_binding, attribute_call, dataclass_decorator, identifier_words, keyword_is_true, last_segment,
    module_statements,
};

// What `dataclass-kw-only` needs to rewrite positional calls of a dataclass: the dataclasses declared without
// `kw_only=True`, their positional fields, and how the file uses names from other modules. `unused-default` reads the
// same uses for function calls, and the methods a class defines, matched against the attribute names the module loads.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Constructors {
    pub(super) dataclasses: Vec<(String, Option<Vec<String>>)>,
    pub(super) uses: Vec<(String, Vec<NameUse>)>,
    // Capitalized names called with positional arguments as attributes of something no import binds, such as
    // `module.Ranked(issue, score)` on a module a test loads from its file path, plus the capitalized names a
    // `getattr(object, "Name")` reads: any dataclass so named may be the one.
    pub(super) dynamic: Vec<String>,
    pub(super) methods: Vec<String>,
    // Plain `name = value` class attributes, which `attribute_loads` counts like methods.
    pub(super) attributes: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct NameUse {
    pub(super) name: String,
    pub(super) written: String,
    pub(super) kind: UseKind,
    pub(super) discarded: bool,
    pub(super) entry: bool,
    // The byte range of the call, which a dataclass report points at; `(0, 0)` for a name that is not a call.
    pub(super) span: (u32, u32),
    pub(super) comments: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) enum UseKind {
    Call(CallShape),
    // A use no rewrite can follow: a call unpacking `*args`, a class pattern matching by position, or the name passed
    // around as a value.
    Opaque,
    Imported,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct CallShape {
    pub(crate) positional: u32,
    pub(crate) keywords: Vec<String>,
    pub(crate) unpacks_mapping: bool,
}

impl CallShape {
    fn of(call: &ast::ExprCall) -> Self {
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

impl Constructors {
    pub(super) fn collect(
        source: &str,
        tokens: &Tokens,
        body: &[Stmt],
        aliases: &FxHashMap<&str, String>,
        from_imports: &[FromImport],
        contexts: &CallContexts,
    ) -> Self {
        let qualify = |expr: &Expr| qualified(UnqualifiedName::from_expr(expr)?.segments(), aliases);
        let mut uses = Uses::new(|expr: &Expr| {
            let full = qualify(expr)?;
            let (module, member) = full.rsplit_once('.')?;
            Some((module.to_owned(), member.to_owned(), UnqualifiedName::from_expr(expr)?.segments().join(".")))
        })
        .with_source(source);
        uses.visit_body(body);
        let mut attribute_uses = Uses::new(|expr: &Expr| {
            let ast::ExprAttribute { value, attr, .. } = expr.as_attribute_expr()?;
            let mut head = value.as_ref();
            while let Expr::Attribute(attribute) = head {
                head = &attribute.value;
            }
            let unbound = head.as_name_expr().is_none_or(|name| !aliases.contains_key(name.id.as_str()));
            (unbound && attr.starts_with(|first: char| first.is_ascii_uppercase())).then(|| attr.to_string())
        });
        attribute_uses.visit_body(body);
        let dynamic: BTreeSet<String> = attribute_uses
            .found
            .into_iter()
            .filter_map(|(name, found)| match found {
                Use::Call(call) if CallKind::of(&call) != CallKind::Keyword => Some(name),
                _ => None,
            })
            .chain(getattr_names(body))
            .chain(unparsed_annotation_names(body))
            .collect();
        let comments = OnceCell::new();
        let mut grouped: BTreeMap<String, BTreeSet<NameUse>> = BTreeMap::new();
        for ((module, name, written), found) in uses.found {
            let (kind, discarded, entry, span, call_comments) = match found {
                Use::Call(call) => {
                    let call: &ast::ExprCall = &call;
                    let (discarded, entry) = contexts.classify(call);
                    let (kind, call_comments) = match CallKind::of(call) {
                        CallKind::Unpacked => (UseKind::Opaque, Vec::new()),
                        CallKind::Keyword => (UseKind::Call(CallShape::of(call)), Vec::new()),
                        CallKind::Positional => (
                            UseKind::Call(CallShape::of(call)),
                            comments_on_lines(source, comments.get_or_init(|| comment_ranges(tokens)), call),
                        ),
                    };
                    let range = call.range();
                    (kind, discarded, entry, (range.start().to_u32(), range.end().to_u32()), call_comments)
                }
                Use::Opaque => (UseKind::Opaque, false, false, (0, 0), Vec::new()),
            };
            grouped.entry(module).or_default().insert(NameUse {
                name,
                written,
                kind,
                discarded,
                entry,
                span,
                comments: call_comments,
            });
        }
        for FromImport { module, names, .. } in from_imports {
            for alias in *names {
                grouped.entry(module.clone()).or_default().insert(NameUse {
                    name: alias.name.to_string(),
                    written: alias_binding(alias).to_owned(),
                    kind: UseKind::Imported,
                    discarded: false,
                    entry: false,
                    span: (0, 0),
                    comments: Vec::new(),
                });
            }
        }
        let mut known: Vec<(&str, Option<Vec<String>>)> = Vec::new();
        let mut dataclasses: Vec<(String, Option<Vec<String>>)> = Vec::new();
        let is_dataclass = |expr: &Expr| qualify(expr).is_some_and(|name| name == "dataclasses.dataclass");
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
        let Members { methods, attributes } = member_names(body);
        Self {
            dataclasses,
            uses: grouped.into_iter().map(|(module, uses)| (module, uses.into_iter().collect())).collect(),
            dynamic: dynamic.into_iter().collect(),
            methods,
            attributes,
        }
    }
}

// Calls whose value a statement drops, and the blocks that run the module as a program. Only the production-call rules
// read them, so other runs leave them empty.
#[derive(Default)]
pub(crate) struct CallContexts {
    discarded: FxHashSet<TextRange>,
    entry: Vec<TextRange>,
}

impl CallContexts {
    pub(crate) fn of(body: &[Stmt], is_main: bool) -> Self {
        Self::new(discarded_calls(body), body, is_main)
    }

    pub(crate) fn new(discarded: FxHashSet<TextRange>, body: &[Stmt], is_main: bool) -> Self {
        Self { discarded, entry: entry_blocks(body, is_main) }
    }

    pub(crate) fn classify(&self, call: &ast::ExprCall) -> (bool, bool) {
        let range = call.range();
        (self.discarded.contains(&range), self.entry.iter().any(|block| block.contains_range(range)))
    }
}

fn discarded_calls(body: &[Stmt]) -> FxHashSet<TextRange> {
    struct Discarded(FxHashSet<TextRange>);
    impl<'a> Visitor<'a> for Discarded {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            self.0.extend(discarded_call(stmt));
            visitor::walk_stmt(self, stmt);
        }
    }
    let mut discarded = Discarded(FxHashSet::default());
    discarded.visit_body(body);
    discarded.0
}

pub(super) fn discarded_call(stmt: &Stmt) -> Option<TextRange> {
    let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
        return None;
    };
    let Expr::Call(call) = value.as_ref() else {
        return None;
    };
    Some(call.range())
}

// A `__main__.py` runs whole as a program; elsewhere only its `if __name__ == "__main__":` blocks do.
fn entry_blocks(body: &[Stmt], is_main: bool) -> Vec<TextRange> {
    if is_main {
        return body.iter().map(Ranged::range).collect();
    }
    body.iter().filter(|stmt| is_main_guard(stmt)).map(Ranged::range).collect()
}

// Every use of the names `matches` recognizes, keyed by what it returns. Annotations, `isinstance` and `issubclass`
// checks, base classes, and attribute reads such as `Name.method` neither call the class nor hand it to code that
// might, so they are not uses. A call in an annotation is a use, and so is one in a string annotation, which is parsed
// when the walk has the source it is in.
pub(crate) struct Uses<'a, K, F> {
    matches: F,
    annotated: bool,
    source: Option<&'a str>,
    pub(crate) found: Vec<(K, Use<'a>)>,
}

pub(crate) enum Use<'a> {
    Call(Cow<'a, ast::ExprCall>),
    Opaque,
}

impl Use<'_> {
    fn into_owned<'b>(self) -> Use<'b> {
        match self {
            Self::Call(call) => Use::Call(Cow::Owned(call.into_owned())),
            Self::Opaque => Use::Opaque,
        }
    }
}

impl<'a, K, F: Fn(&Expr) -> Option<K>> Uses<'a, K, F> {
    pub(crate) fn new(matches: F) -> Self {
        Self { matches, annotated: false, source: None, found: Vec::new() }
    }

    pub(crate) fn with_source(mut self, source: &'a str) -> Self {
        self.source = Some(source);
        self
    }

    // A string annotation is code the checker reads as an expression. Its ranges are the file's, since parsing starts
    // at the string's contents, but the expression it parses is dropped with the walk, so its calls are owned.
    fn visit_string_annotation(&mut self, literal: &ast::ExprStringLiteral) {
        let (Some(source), Some(string)) = (self.source, literal.as_single_part_string()) else {
            return;
        };
        let Ok(parsed) = parse_string_annotation(source, string) else {
            return;
        };
        let matches: &dyn Fn(&Expr) -> Option<K> = &self.matches;
        let mut inner = Uses { matches, annotated: true, source: self.source, found: Vec::new() };
        inner.visit_expr(parsed.expr());
        let found = inner.found;
        self.found.extend(found.into_iter().map(|(key, found)| (key, found.into_owned())));
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

    fn visit_annotation(&mut self, annotation: &'a Expr) {
        self.annotated = true;
        self.visit_expr(annotation);
        self.annotated = false;
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(key) = (self.matches)(expr) {
            if !self.annotated {
                self.found.push((key, Use::Opaque));
            }
            return;
        }
        // A call in an annotation, as in `Annotated[str, Doc("...")]`, runs code, so its arguments are values.
        if self.annotated
            && let Expr::Call(_) = expr
        {
            self.annotated = false;
            self.visit_expr(expr);
            self.annotated = true;
            return;
        }
        match expr {
            Expr::Call(call) if let Some(key) = (self.matches)(&call.func) => {
                self.found.push((key, Use::Call(Cow::Borrowed(call))));
                self.visit_arguments(&call.arguments);
            }
            // The metadata of `Annotated[int, ...]` is values, which a class can be passed as and called by position.
            Expr::Subscript(ast::ExprSubscript { value, slice, .. })
                if self.annotated && last_segment(value) == Some("Annotated") =>
            {
                let arguments = match slice.as_ref() {
                    Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.as_slice(),
                    single => std::slice::from_ref(single),
                };
                for (position, argument) in arguments.iter().enumerate() {
                    self.annotated = position == 0;
                    self.visit_expr(argument);
                }
                self.annotated = true;
            }
            Expr::StringLiteral(literal) if self.annotated => self.visit_string_annotation(literal),
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

// The capitalized names a `getattr(object, "Name")` reads: a class read this way may be called by position anywhere.
fn getattr_names(body: &[Stmt]) -> Vec<String> {
    let mut reached = ExprCollector::new(|expr: &Expr| {
        let (_, attribute) = attribute_call(expr, &["getattr"])?;
        let name = attribute.as_string_literal_expr()?.value.to_str();
        name.starts_with(|first: char| first.is_ascii_uppercase()).then(|| name.to_owned())
    });
    reached.visit_body(body);
    reached.found
}

// The words of the implicitly concatenated string annotations of the module. Ruff parses such a string without the
// offsets a rewrite of its calls needs, so a class one of the words spells is taken as used in no rewritable way.
fn unparsed_annotation_names(body: &[Stmt]) -> Vec<String> {
    let mut strings = Unparsed(Vec::new());
    strings.visit_body(body);
    strings.0
}

struct Unparsed(Vec<String>);

impl<'a> Visitor<'a> for Unparsed {
    fn visit_annotation(&mut self, annotation: &'a Expr) {
        let mut strings = ExprCollector::new(|expr: &'a Expr| {
            expr.as_string_literal_expr().filter(|literal| literal.as_single_part_string().is_none())
        });
        strings.visit_expr(annotation);
        for literal in strings.found {
            for part in &literal.value {
                let words = identifier_words(&part.value).filter(|word| !word.is_empty());
                self.0.extend(words.map(str::to_owned));
            }
        }
    }
}

fn comments_on_lines(source: &str, comments: &CommentRanges, call: &ast::ExprCall) -> Vec<String> {
    comments
        .comments_in_range(source.full_lines_range(call.range()))
        .iter()
        .map(|comment| source[*comment].to_owned())
        .collect()
}

fn comment_ranges(tokens: &Tokens) -> CommentRanges {
    CommentRanges::new(tokens.iter().filter(|token| token.kind() == TokenKind::Comment).map(Ranged::range).collect())
}

fn subscripted(expr: &Expr) -> &Expr {
    expr.as_subscript_expr().map_or(expr, |subscript| &subscript.value)
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
    let mut seen: FxHashSet<String> = fields.iter().cloned().collect();
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
        if in_init && !by_keyword && seen.insert(name.to_owned()) {
            fields.push(name.to_owned());
        }
    }
    Some(fields)
}

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
    Members { methods, attributes }
}

struct Members {
    methods: Vec<String>,
    attributes: Vec<String>,
}

#[derive(Default)]
pub(super) struct RunCalls {
    // Includes a dataclass imported through a module that passes it on, which no rewrite follows.
    opaque: FxHashMap<FileId, FxHashSet<String>>,
    star_imported: FxHashSet<FileId>,
    dataclass_calls: FxHashMap<FileId, FxHashMap<String, Vec<CallSite>>>,
    functions: FxHashMap<FileId, FxHashMap<String, CallSites>>,
    method_loads: FxHashMap<FileId, FxHashMap<String, u32>>,
    test_support: FxHashSet<FileId>,
}

type Declared<'f> = &'f [(String, Option<Vec<String>>)];

pub(super) struct FileUses<'f> {
    pub(super) caller: FileId,
    pub(super) path: PathBuf,
    // A digest of the file's source, which a cached report that points into it must match.
    pub(super) digest: u64,
    pub(super) production: bool,
    pub(super) modules: Vec<(FileId, &'f [NameUse])>,
}

impl RunCalls {
    // `defined` tells a re-export from a definition. `hidden` lists the names some file reads off a module under a
    // computed name, so any function so named may be called. A library's public dataclasses are left out, so no call to
    // them is rewritten or reported.
    pub(super) fn index<'f>(
        declared: impl Iterator<Item = (FileId, Declared<'f>)>,
        uses: &[FileUses],
        imported: &FxHashMap<FileId, FxHashSet<String>>,
        dynamic: &FxHashSet<&str>,
        defined: &FxHashMap<FileId, FxHashSet<&str>>,
        hidden: &FxHashSet<&str>,
        library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    ) -> Self {
        let is_library =
            |file: FileId, class: &str| library_api.get(&file).is_some_and(|names| names.contains_key(class));
        let declared: FxHashMap<FileId, Declared<'f>> =
            declared.filter(|(file, classes)| classes.iter().any(|(class, _)| !is_library(*file, class))).collect();
        let mut calls = Self::default();
        for (file, classes) in &declared {
            let called =
                classes.iter().filter(|(class, _)| dynamic.contains(class.as_str()) && !is_library(*file, class));
            for (class, _) in called {
                calls.opaque.entry(*file).or_default().insert(class.clone());
            }
        }
        let mut reexported: FxHashSet<String> = FxHashSet::default();
        for FileUses { caller, path, digest, production, modules } in uses {
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
                for NameUse { name, written, kind, span, comments, .. } in *used {
                    if *kind == UseKind::Imported && name == "*" {
                        calls.star_imported.insert(*file);
                        continue;
                    }
                    let Some((_, fields)) =
                        classes.iter().find(|(class, _)| class == name && !is_library(*file, class))
                    else {
                        continue;
                    };
                    // A call passing fields by position is a site the dataclass's report names; one that passes more
                    // than the class takes is not one a rewrite could follow.
                    let opaque = match kind {
                        UseKind::Imported => imported.get(caller).is_some_and(|names| names.contains(written)),
                        UseKind::Call(shape) => {
                            if shape.positional > 0 {
                                let site = CallSite {
                                    caller: path.clone(),
                                    start: span.0,
                                    end: span.1,
                                    digest: *digest,
                                    comments: comments.clone(),
                                };
                                calls
                                    .dataclass_calls
                                    .entry(*file)
                                    .or_default()
                                    .entry(name.clone())
                                    .or_default()
                                    .push(site);
                            }
                            shape.positional > 0 && fields.as_ref().is_none_or(|f| shape.positional as usize > f.len())
                        }
                        UseKind::Opaque => true,
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

    // An entry point or a dotted string naming a function hides its arguments.
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
            dataclass_calls: id.and_then(|id| self.dataclass_calls.get(&id)),
            functions: id.and_then(|id| self.functions.get(&id)),
            method_loads: id.and_then(|id| self.method_loads.get(&id)),
            test_support: id.is_some_and(|id| self.test_support.contains(&id)),
        }
    }

    pub(super) fn set_test_support(&mut self, files: FxHashSet<FileId>) {
        self.test_support = files;
    }
}

// Every call of a function the run makes, and whether some use of it hides its arguments. Production calls are kept
// apart, as the test-only rules compare them with the calls of tests.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct CallSites {
    pub(crate) opaque: bool,
    pub(crate) shapes: BTreeSet<CallShape>,
    pub(crate) production: BTreeSet<CallShape>,
    pub(crate) production_reads: bool,
    // Whether code running the module as a program calls it: its arguments and value belong to the command line, not
    // to the run.
    pub(crate) entry: bool,
}

impl CallSites {
    fn add_shape(&mut self, shape: CallShape) {
        self.shapes.insert(shape);
    }

    pub(crate) fn add_call(&mut self, call: &ast::ExprCall) {
        match CallKind::of(call) {
            CallKind::Unpacked => self.opaque = true,
            _ => self.add_shape(CallShape::of(call)),
        }
    }

    pub(crate) fn add_production_call(&mut self, call: &ast::ExprCall, discarded: bool, entry: bool) {
        if CallKind::of(call) == CallKind::Unpacked {
            self.opaque = true;
            return;
        }
        let shape = CallShape::of(call);
        self.add_production(&shape, !discarded, entry);
        self.shapes.insert(shape);
    }

    fn add_production(&mut self, shape: &CallShape, reads: bool, entry: bool) {
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

// A call that passes fields by position to a dataclass of another file: the file it is in, its byte range, and the
// digest of that file's source, so a report that names it is kept only while the file is unchanged.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct CallSite {
    pub(crate) caller: PathBuf,
    pub(crate) start: u32,
    pub(crate) end: u32,
    digest: u64,
    pub(crate) comments: Vec<String>,
}

impl CacheKey for CallSite {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.caller.cache_key(state);
        self.start.cache_key(state);
        self.end.cache_key(state);
        self.digest.cache_key(state);
        self.comments.cache_key(state);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FileCalls<'a> {
    opaque: Option<&'a FxHashSet<String>>,
    star_imported: bool,
    dataclass_calls: Option<&'a FxHashMap<String, Vec<CallSite>>>,
    functions: Option<&'a FxHashMap<String, CallSites>>,
    method_loads: Option<&'a FxHashMap<String, u32>>,
    test_support: bool,
}

impl<'a> FileCalls<'a> {
    pub(crate) fn is_test_support(&self) -> bool {
        self.test_support
    }

    // A star import hides which names the importing file uses, so it could use any dataclass in any way.
    pub(crate) fn has_opaque_use(&self, name: &str) -> bool {
        self.star_imported || self.opaque.is_some_and(|names| names.contains(name))
    }

    pub(crate) fn positional_calls(&self, name: &str) -> &'a [CallSite] {
        self.dataclass_calls.and_then(|calls| calls.get(name)).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn call_sites(&self, name: &str) -> Option<&'a CallSites> {
        self.functions?.get(name)
    }

    pub(crate) fn attribute_loads(&self, name: &str) -> u32 {
        self.method_loads.and_then(|loads| loads.get(name)).copied().unwrap_or_default()
    }
}

impl CacheKey for FileCalls<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.opaque.cache_key(state);
        self.star_imported.cache_key(state);
        self.dataclass_calls.cache_key(state);
        self.functions.cache_key(state);
        self.method_loads.cache_key(state);
        self.test_support.cache_key(state);
    }
}
