use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt};
use ruff_python_trivia::CommentRanges;
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use super::facts::{FromImport, qualified};
use crate::lint::noqa;
use crate::lint::registry::Rule;

// What `dataclass-kw-only` needs to rewrite every positional call of a dataclass it makes keyword-only: the dataclasses
// a file declares without `kw_only=True`, with the fields their `__init__` takes by position, and how the file uses
// names from other modules.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Constructors {
    pub(super) dataclasses: Vec<(String, Option<Vec<String>>)>,
    pub(super) uses: Vec<(String, Vec<NameUse>)>,
    // The capitalized names called with positional arguments as attributes of something no import binds, such as
    // `module.Ranked(issue, score)` on a module a test loads from its file path; any dataclass so named may be the one.
    pub(super) dynamic: Vec<String>,
}

// A name used from a module, as the file spells it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct NameUse {
    pub(super) name: String,
    pub(super) written: String,
    pub(super) kind: UseKind,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) enum UseKind {
    // A call passing this many arguments by position, and no `*args`.
    Positional(u32),
    // A use no rewrite can follow: a call unpacking `*args`, a class pattern matching by position, or the name passed
    // around as a value.
    Opaque,
    // `from module import name as written`, or `from module import *` with `*` for the name.
    Imported,
}

impl Constructors {
    pub(super) fn collect(
        source: &str,
        tokens: &Tokens,
        body: &[Stmt],
        aliases: &HashMap<&str, String>,
        from_imports: &[FromImport],
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
            let kind = match found {
                Use::Call(call) => match CallKind::of(call) {
                    CallKind::Keyword => continue,
                    CallKind::Positional
                        if is_suppressed(source, comments.get_or_init(|| comment_ranges(tokens)), call) =>
                    {
                        UseKind::Opaque
                    }
                    CallKind::Positional => {
                        UseKind::Positional(u32::try_from(call.arguments.args.len()).unwrap_or(u32::MAX))
                    }
                    CallKind::Unpacked => UseKind::Opaque,
                },
                Use::Opaque => UseKind::Opaque,
            };
            grouped.entry(module).or_default().insert(NameUse { name, written, kind });
        }
        for FromImport { module, names, .. } in from_imports {
            for alias in *names {
                let written = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
                let name = alias.name.to_string();
                grouped.entry(module.clone()).or_default().insert(NameUse { name, written, kind: UseKind::Imported });
            }
        }
        let is_dataclass = |expr: &Expr| qualify(expr).is_some_and(|name| name == "dataclasses.dataclass");
        let mut known: Vec<(&str, Option<Vec<String>>)> = Vec::new();
        let mut dataclasses: Vec<(String, Option<Vec<String>>)> = Vec::new();
        for class in body.iter().filter_map(Stmt::as_class_def_stmt) {
            let Some(arguments) = dataclass_arguments(class, &is_dataclass) else {
                continue;
            };
            let base_fields =
                |name: &str| known.iter().find(|(base, _)| *base == name).and_then(|(_, fields)| fields.clone());
            let fields = init_fields(class, arguments, &base_fields);
            if !is_keyword_only(arguments) {
                dataclasses.push((class.name.to_string(), fields.clone()));
            }
            known.push((class.name.as_str(), fields));
        }
        Self {
            dataclasses,
            uses: grouped.into_iter().map(|(module, uses)| (module, uses.into_iter().collect())).collect(),
            dynamic: dynamic.into_iter().collect(),
        }
    }
}

// A suppression comment on the call keeps its own `dataclass-kw-only` report from rewriting it.
fn is_suppressed(source: &str, comments: &CommentRanges, call: &ast::ExprCall) -> bool {
    comments
        .comments_in_range(source.full_lines_range(call.range()))
        .iter()
        .any(|comment| noqa::suppresses(&source[*comment], Rule::DataclassKwOnly.code()))
}

fn comment_ranges(tokens: &Tokens) -> CommentRanges {
    CommentRanges::new(tokens.iter().filter(|token| token.kind() == TokenKind::Comment).map(Ranged::range).collect())
}

// Over the run: how other files use each file's dataclasses, and for each file the fields of the dataclasses it calls
// with positional arguments, by how it spells the call.
#[derive(Default)]
pub struct DataclassCalls {
    // Per file, its dataclasses another file uses in a way no rewrite follows, which includes importing one from a
    // module that passes it on to others.
    opaque: HashMap<PathBuf, HashSet<String>>,
    star_imported: HashSet<PathBuf>,
    fields: HashMap<PathBuf, HashMap<String, Vec<String>>>,
}

type Declared<'f> = &'f [(String, Option<Vec<String>>)];

// A file, whether its own reports rewrite its positional calls, and the files its names resolve to with the names it
// uses from each.
pub(super) type FileUses<'f> = (PathBuf, bool, Vec<(PathBuf, &'f [NameUse])>);

impl DataclassCalls {
    pub(super) fn index<'f>(
        declared: impl Iterator<Item = (PathBuf, Declared<'f>)>,
        uses: &[FileUses],
        imported: &HashMap<PathBuf, HashSet<String>>,
        dynamic: &HashSet<&str>,
    ) -> Self {
        let declared: HashMap<PathBuf, Declared<'f>> = declared.filter(|(_, classes)| !classes.is_empty()).collect();
        let mut calls = Self::default();
        for (file, classes) in &declared {
            let called = classes.iter().filter(|(class, _)| dynamic.contains(class.as_str()));
            for (class, _) in called {
                calls.opaque.entry(file.clone()).or_default().insert(class.clone());
            }
        }
        for (caller, rewrites_calls, modules) in uses {
            for (file, uses) in modules {
                let Some(classes) = declared.get(file) else {
                    continue;
                };
                for NameUse { name, written, kind } in *uses {
                    if *kind == UseKind::Imported && name == "*" {
                        calls.star_imported.insert(file.clone());
                        continue;
                    }
                    let Some((_, fields)) = classes.iter().find(|(class, _)| class == name) else {
                        continue;
                    };
                    let opaque = match (kind, fields) {
                        (UseKind::Imported, _) => imported.get(caller).is_some_and(|names| names.contains(written)),
                        (UseKind::Positional(count), Some(fields))
                            if *rewrites_calls && *count as usize <= fields.len() =>
                        {
                            calls.fields.entry(caller.clone()).or_default().insert(written.clone(), fields.clone());
                            false
                        }
                        _ => true,
                    };
                    if opaque {
                        calls.opaque.entry(file.clone()).or_default().insert(name.clone());
                    }
                }
            }
        }
        calls
    }

    pub(super) fn file(&self, path: &Path) -> FileCalls<'_> {
        FileCalls {
            opaque: self.opaque.get(path),
            star_imported: self.star_imported.contains(path),
            fields: self.fields.get(path),
        }
    }
}

// What the index knows of one file's dataclass calls: how other files use its dataclasses, and the dataclasses of
// other files it calls by position.
#[derive(Clone, Copy, PartialEq)]
pub struct FileCalls<'a> {
    opaque: Option<&'a HashSet<String>>,
    star_imported: bool,
    fields: Option<&'a HashMap<String, Vec<String>>>,
}

impl<'a> FileCalls<'a> {
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
}

impl CacheKey for FileCalls<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.opaque.cache_key(state);
        self.star_imported.cache_key(state);
        self.fields.cache_key(state);
    }
}

// The decorator's arguments when `class` is a dataclass, an empty list for a bare `@dataclass`.
pub(crate) fn dataclass_arguments<'a>(
    class: &'a ast::StmtClassDef,
    is_dataclass: &impl Fn(&Expr) -> bool,
) -> Option<&'a [ast::Keyword]> {
    class.decorator_list.iter().find_map(|decorator| match &decorator.expression {
        Expr::Call(ast::ExprCall { func, arguments, .. }) if is_dataclass(func) => Some(&arguments.keywords[..]),
        other if is_dataclass(other) => Some(&[][..]),
        _ => None,
    })
}

pub(crate) fn is_keyword_only(keywords: &[ast::Keyword]) -> bool {
    keyword(keywords, "kw_only")
        .is_some_and(|value| value.as_boolean_literal_expr().is_some_and(|literal| literal.value))
}

fn keyword<'a>(keywords: &'a [ast::Keyword], name: &str) -> Option<&'a Expr> {
    keywords
        .iter()
        .find(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == name))
        .map(|keyword| &keyword.value)
}

// The fields the generated `__init__` takes by position, in order; `None` when simplint cannot tell them, as when the
// class writes its own `__init__` or inherits from a class it does not know. A base dataclass of the same module,
// whose fields `base_fields` gives, comes first, as dataclasses order inherited fields.
pub(crate) fn init_fields(
    class: &ast::StmtClassDef,
    keywords: &[ast::Keyword],
    base_fields: &impl Fn(&str) -> Option<Vec<String>>,
) -> Option<Vec<String>> {
    if keyword(keywords, "init")
        .is_some_and(|init| !init.as_boolean_literal_expr().is_some_and(|literal| literal.value))
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
    let keyword_only = is_keyword_only(keywords);
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
                Some(&arguments.keywords[..])
            }
            _ => None,
        });
        let flag = |option: &str, default: bool| match options.and_then(|keywords| keyword(keywords, option)) {
            None => Some(default),
            Some(value) => value.as_boolean_literal_expr().map(|literal| literal.value),
        };
        if options.is_some_and(|keywords| keywords.iter().any(|keyword| keyword.arg.is_none())) {
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
