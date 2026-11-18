use std::borrow::Cow;
use std::fmt;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_ast::{self as ast, ConversionFlag, Expr, InterpolatedStringElement, Stmt, StringFlags};
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rules::edits::above_comments;
use crate::semantic::future_import;
use crate::syntax::statement_index;
use crate::syntax::{alias_binding, alias_root_binding, is_type_checking_name};

// The modules a normal start of CPython 3.15 has loaded before any user code runs, so a lazy import of one defers
// nothing. Linted code can run on a different platform than the linter.
const LOADED_AT_STARTUP: [&str; 16] = [
    "abc",
    "builtins",
    "codecs",
    "encodings",
    "encodings.aliases",
    "encodings.utf_8",
    "errno",
    "genericpath",
    "marshal",
    "os",
    "os.path",
    "site",
    "stat",
    "sys",
    "time",
    "zipimport",
];

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum Module {
    Absolute(String),
    // `level` packages up from the file, as `from ..name import x` or `f"{__spec__.parent}.name"` spell it.
    Relative { level: u32, name: String },
}

impl Module {
    // CPython compares `__lazy_modules__` with absolute names, so a relative module resolves against the file's package
    // when the file has one.
    pub(super) fn resolve(self, module_path: Option<&[String]>) -> Self {
        let Self::Relative { level, name } = self else {
            return self;
        };
        let resolved = resolve_imported_module_path(level, (!name.is_empty()).then_some(name.as_str()), module_path)
            .map(Cow::into_owned);
        resolved.map_or(Self::Relative { level, name }, Self::Absolute)
    }

    pub(super) fn is_loaded_at_startup(&self) -> bool {
        matches!(self, Self::Absolute(name) if LOADED_AT_STARTUP.contains(&name.as_str()))
    }

    // The entry that names the module. A relative module keeps its form through `__spec__.parent`, the package the
    // file runs in, so the entry follows the package when it is renamed or vendored.
    pub(super) fn entry(&self, quote: char) -> String {
        let other = if quote == '"' { '\'' } else { '"' };
        match self {
            Self::Absolute(name) => format!("{quote}{name}{quote}"),
            Self::Relative { level, name } => {
                let parent = if *level == 1 {
                    "__spec__.parent".to_owned()
                } else {
                    format!("__spec__.parent.rsplit({other}.{other}, {})[0]", level - 1)
                };
                let suffix = if name.is_empty() { String::new() } else { format!(".{name}") };
                format!("f{quote}{{{parent}}}{suffix}{quote}")
            }
        }
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute(name) => f.write_str(name),
            Self::Relative { level, name } => write!(f, "{}{name}", ".".repeat(*level as usize)),
        }
    }
}

pub(super) struct Entry<'a> {
    pub(super) expr: &'a Expr,
    pub(super) module: Module,
    // Written as a relative name such as `".helper"`, which no import's absolute name equals.
    pub(super) written_relative: bool,
}

pub(super) enum Declaration<'a> {
    Absent,
    // Built at run time, or changed after its assignment, so the modules it holds are unknown.
    Unknown,
    Listed(LazyList<'a>),
}

pub(super) struct LazyList<'a> {
    pub(super) stmt: &'a Stmt,
    pub(super) value: &'a Expr,
    pub(super) entries: Vec<Entry<'a>>,
}

// A comment at the end of an item's line moves with the item.
pub(super) struct Item {
    pub(super) text: String,
    pub(super) comment: Option<String>,
}

// The module-level `__lazy_modules__`, read only from a single assignment of literal entries, since any other mention
// of the name may change what it holds.
pub(super) fn declaration<'a>(checker: &Checker<'a>) -> Declaration<'a> {
    let mentions = checker.name_token_count("__lazy_modules__");
    if mentions == 0 {
        return Declaration::Absent;
    }
    let Some((stmt, value)) = checker.body().iter().find_map(|stmt| Some((stmt, assigned_value(stmt)?))) else {
        return Declaration::Unknown;
    };
    let (Expr::List(ast::ExprList { elts, .. })
    | Expr::Tuple(ast::ExprTuple { elts, .. })
    | Expr::Set(ast::ExprSet { elts, .. })) = value
    else {
        return Declaration::Unknown;
    };
    match elts.iter().map(entry).collect::<Option<Vec<_>>>() {
        Some(entries) if mentions == 1 => Declaration::Listed(LazyList { stmt, value, entries }),
        _ => Declaration::Unknown,
    }
}

fn assigned_value(stmt: &Stmt) -> Option<&Expr> {
    let (target, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match targets.as_slice() {
            [target] => (target, value),
            _ => return None,
        },
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => (target.as_ref(), value),
        _ => return None,
    };
    target.as_name_expr().filter(|name| name.id.as_str() == "__lazy_modules__").map(|_| value.as_ref())
}

fn entry(expr: &Expr) -> Option<Entry<'_>> {
    match expr {
        Expr::StringLiteral(literal) => {
            let value = literal.value.to_str();
            let name = value.trim_start_matches('.');
            let level = u32::try_from(value.len() - name.len()).unwrap_or(u32::MAX);
            let written_relative = level > 0;
            let module = if written_relative {
                Module::Relative { level, name: name.to_owned() }
            } else {
                Module::Absolute(value.to_owned())
            };
            Some(Entry { expr, module, written_relative })
        }
        Expr::FString(fstring) => Some(Entry { expr, module: spec_module(fstring)?, written_relative: false }),
        _ => None,
    }
}

// `f"{__spec__.parent}.name"`, the spelling flake8-lazy gives a relative import, or `f"{__spec__.parent}"` for the
// package itself.
fn spec_module(fstring: &ast::ExprFString) -> Option<Module> {
    let mut parts = fstring.value.iter();
    let (Some(ast::FStringPartRef::FString(part)), None) = (parts.next(), parts.next()) else {
        return None;
    };
    let (parent, name) = match &*part.elements {
        [InterpolatedStringElement::Interpolation(parent)] => (parent, ""),
        [InterpolatedStringElement::Interpolation(parent), InterpolatedStringElement::Literal(rest)] => {
            (parent, rest.value.strip_prefix('.').filter(|name| !name.is_empty())?)
        }
        _ => return None,
    };
    if parent.debug_text.is_some() || parent.conversion != ConversionFlag::None || parent.format_spec.is_some() {
        return None;
    }
    Some(Module::Relative { level: spec_parent_level(&parent.expression)?, name: name.to_owned() })
}

// `__spec__.parent` is one package up; `__spec__.parent.rsplit(".", n)[0]`, also guarded as
// `(__spec__.parent or "")`, walks `n` packages further.
fn spec_parent_level(expr: &Expr) -> Option<u32> {
    if is_spec_parent(expr) {
        return Some(1);
    }
    let Expr::Subscript(ast::ExprSubscript { value, slice, .. }) = expr else {
        return None;
    };
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = value.as_ref() else {
        return None;
    };
    let Expr::Attribute(ast::ExprAttribute { value: receiver, attr, .. }) = func.as_ref() else {
        return None;
    };
    let [Expr::StringLiteral(separator), Expr::NumberLiteral(ast::ExprNumberLiteral { value: count, .. })] =
        &*arguments.args
    else {
        return None;
    };
    let guarded = matches!(receiver.as_ref(), Expr::BoolOp(ast::ExprBoolOp { op: ast::BoolOp::Or, values, .. })
        if matches!(values.as_slice(), [parent, Expr::StringLiteral(empty)]
            if is_spec_parent(parent) && empty.value.is_empty()));
    let first = slice.as_number_literal_expr().and_then(|index| index.value.as_int()).and_then(ast::Int::as_u8);
    if attr.as_str() != "rsplit"
        || !arguments.keywords.is_empty()
        || separator.value.to_str() != "."
        || first != Some(0)
        || !(guarded || is_spec_parent(receiver))
    {
        return None;
    }
    count.as_int().and_then(ast::Int::as_u8).map(|count| u32::from(count) + 1)
}

fn is_spec_parent(expr: &Expr) -> bool {
    matches!(expr, Expr::Attribute(ast::ExprAttribute { value, attr, .. })
        if attr.as_str() == "parent" && value.as_name_expr().is_some_and(|name| name.id.as_str() == "__spec__"))
}

impl LazyList<'_> {
    pub(super) fn quote(&self, checker: &Checker) -> char {
        self.entries
            .iter()
            .find_map(|entry| entry.expr.as_string_literal_expr()?.value.iter().next())
            .map_or_else(|| checker.stylist.quote().as_char(), |part| part.flags.quote_style().as_char())
    }

    // The items with the comments that end their lines; none when a comment sits anywhere else in the list, where
    // rewriting it would lose the comment.
    pub(super) fn items(&self, checker: &Checker) -> Option<Vec<Item>> {
        let source = checker.source;
        let items: Vec<Item> = self
            .entries
            .iter()
            .map(|entry| {
                let range = entry.expr.range();
                let comment = checker.trailing_comment(range).filter(|comment| {
                    !has_leading_content(range.start(), source)
                        && matches!(source[TextRange::new(range.end(), comment.start())].trim(), "" | ",")
                });
                Item {
                    text: checker.text(range).to_owned(),
                    comment: comment.map(|comment| checker.text(comment).to_owned()),
                }
            })
            .collect();
        let carried = items.iter().filter(|item| item.comment.is_some()).count();
        (checker.comment_ranges().comments_in_range(self.value.range()).len() == carried).then_some(items)
    }

    pub(super) fn replacement(&self, checker: &Checker, items: &[Item]) -> Option<Edit> {
        let brackets = match self.value {
            Expr::List(_) => ("[", "]"),
            Expr::Set(_) if !items.is_empty() => ("{", "}"),
            Expr::Tuple(tuple) if tuple.parenthesized => ("(", ")"),
            _ => return None,
        };
        let source = checker.source;
        let layout = Layout {
            brackets,
            single_tuple: self.value.is_tuple_expr() && items.len() == 1,
            base: checker.indentation(self.stmt),
            used: usize::from(self.value.start() - source.line_start(self.value.start()))
                + usize::from(source.line_end(self.value.end()) - self.value.end()),
            spans_lines: source.contains_line_break(self.value.range()),
        };
        Some(Edit::range_replacement(layout.text(checker, items), self.value.range()))
    }
}

pub(super) struct Layout<'a> {
    pub(super) brackets: (&'a str, &'a str),
    pub(super) single_tuple: bool,
    pub(super) base: &'a str,
    // The width the line holds besides the list.
    pub(super) used: usize,
    pub(super) spans_lines: bool,
}

impl Layout<'_> {
    // The list holding `items`, laid out as `ruff format` keeps it: one item a line, each with its comment, when the
    // list spans lines, carries a comment, or does not fit its line.
    pub(super) fn text(&self, checker: &Checker, items: &[Item]) -> String {
        let (open, close) = self.brackets;
        let inline = format!(
            "{open}{}{}{close}",
            items.iter().map(|item| item.text.as_str()).collect::<Vec<_>>().join(", "),
            if self.single_tuple { "," } else { "" }
        );
        if !self.spans_lines
            && items.iter().all(|item| item.comment.is_none())
            && self.used + inline.len() <= checker.settings.from_ruff.line_width
        {
            return inline;
        }
        let indent = checker.stylist.indentation().as_str();
        let eol = checker.line_ending();
        let lines = items.iter().fold(String::new(), |text, item| {
            let comment = item.comment.as_ref().map_or_else(String::new, |comment| format!("  {comment}"));
            text + self.base + indent + &item.text + "," + &comment + eol
        });
        format!("{open}{eol}{lines}{}{close}", self.base)
    }
}

// Plain names sort first, then the `__spec__.parent` entries, as a line sort in an editor puts `"` before `f"`.
pub(super) fn sort_key(text: &str) -> (bool, &str) {
    let body = text.trim_start_matches(|char: char| char.is_ascii_alphabetic());
    let fstring = text[..text.len() - body.len()].contains(['f', 'F']);
    let inner = ["\"\"\"", "'''", "\"", "'"]
        .into_iter()
        .find_map(|quote| body.strip_prefix(quote).and_then(|rest| rest.strip_suffix(quote)))
        .unwrap_or(body);
    (fstring, inner)
}

pub(super) struct ModuleImport<'a> {
    pub(super) stmt: &'a Stmt,
    // Directly in the module body, rather than in a block.
    pub(super) top: bool,
    // Whether `__lazy_modules__` defers it: CPython keeps an import in the body or a handler of a `try` eager.
    pub(super) governed: bool,
    // Under `if TYPE_CHECKING:`, which never runs.
    pub(super) dead: bool,
}

// The imports that run while the module loads, in source order; imports in a function or class body run later.
pub(super) fn module_imports(body: &[Stmt]) -> Vec<ModuleImport<'_>> {
    let mut imports = Vec::new();
    collect_imports(body, true, true, false, &mut imports);
    imports
}

fn collect_imports<'a>(body: &'a [Stmt], top: bool, governed: bool, dead: bool, imports: &mut Vec<ModuleImport<'a>>) {
    for stmt in body {
        match stmt {
            Stmt::Import(_) | Stmt::ImportFrom(_) => imports.push(ModuleImport { stmt, top, governed, dead }),
            Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => {
                collect_imports(body, false, governed, dead || is_type_checking_name(test), imports);
                for clause in elif_else_clauses {
                    collect_imports(&clause.body, false, governed, dead, imports);
                }
            }
            Stmt::With(ast::StmtWith { body, .. }) => collect_imports(body, false, governed, dead, imports),
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                collect_imports(body, false, governed, dead, imports);
                collect_imports(orelse, false, governed, dead, imports);
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                for case in cases {
                    collect_imports(&case.body, false, governed, dead, imports);
                }
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                collect_imports(body, false, false, dead, imports);
                for handler in handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    collect_imports(&handler.body, false, false, dead, imports);
                }
                // The `else` of a `try` with a `finally` still runs inside the `try`.
                collect_imports(orelse, false, governed && finalbody.is_empty(), dead, imports);
                collect_imports(finalbody, false, governed, dead, imports);
            }
            _ => {}
        }
    }
}

pub(super) struct Imported<'a> {
    pub(super) module: Module,
    pub(super) binding: &'a str,
    // Where a report on the import points: the alias of an `import`, the module of a `from` import.
    pub(super) range: TextRange,
    pub(super) from_import: bool,
}

// The module an import loads with each name it binds. A star import and a `__future__` import have no lazy form.
pub(super) fn imported_modules(stmt: &Stmt) -> Vec<Imported<'_>> {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => names
            .iter()
            .map(|alias| Imported {
                module: Module::Absolute(alias.name.to_string()),
                binding: alias_root_binding(alias),
                range: alias.range,
                from_import: false,
            })
            .collect(),
        Stmt::ImportFrom(ast::StmtImportFrom { module, names, level, range, .. })
            if future_import(stmt).is_none() && names.iter().all(|alias| alias.name.as_str() != "*") =>
        {
            let name = module.as_ref().map_or_else(String::new, ToString::to_string);
            let range = module.as_ref().map_or(*range, Ranged::range);
            let module = if *level == 0 { Module::Absolute(name) } else { Module::Relative { level: *level, name } };
            names
                .iter()
                .map(|alias| Imported {
                    module: module.clone(),
                    binding: alias_binding(alias),
                    range,
                    from_import: true,
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

// The edit that turns `lazy import x` back into `import x`. A lazy statement starts with the soft keyword, which only
// whitespace and line continuations separate from `import` or `from`.
pub(super) fn lazy_keyword_removal(checker: &Checker, stmt: &Stmt) -> Edit {
    let rest = checker.text(stmt)["lazy".len()..].trim_start_matches([' ', '\t', '\x0c', '\\', '\r', '\n']);
    Edit::deletion(stmt.start(), stmt.end() - TextSize::of(rest))
}

// Where a list goes above `first`: above the comments that lead into it, unless a statement before it shares that
// line, as `from __future__ import annotations; import json` does.
pub(super) fn list_position(checker: &Checker, first: &Stmt) -> Option<TextSize> {
    let position = above_comments(checker, first);
    let body = checker.body();
    let index = statement_index(body, first.start());
    index.checked_sub(1).is_none_or(|previous| body[previous].end() <= position).then_some(position)
}

// The modules a statement imports, each once, as `from x import a, b` names `x` for both names.
pub(super) fn distinct_modules(stmt: &Stmt) -> Vec<Imported<'_>> {
    let mut modules: Vec<Imported> = Vec::new();
    for imported in imported_modules(stmt) {
        if modules.iter().all(|seen| seen.module != imported.module) {
            modules.push(imported);
        }
    }
    modules
}

// The packages that hold the file, which Python loads before it runs.
pub(super) fn enclosing_packages(module_path: Option<&[String]>) -> Vec<Module> {
    let parts = module_path.map_or(&[][..], |path| &path[..path.len().saturating_sub(1)]);
    (1..=parts.len()).map(|end| Module::Absolute(parts[..end].join("."))).collect()
}
