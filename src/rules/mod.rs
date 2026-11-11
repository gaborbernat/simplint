pub(crate) mod api;
pub(crate) mod call_sites;
pub(crate) mod classes;
pub(crate) mod comments;
pub(crate) mod docstrings;
pub(crate) mod edits;
pub(crate) mod imports;
pub(crate) mod layout;
pub(crate) mod logic;
pub(crate) mod module_variable;
pub(crate) mod naming;
mod rename;
pub(crate) mod testing;
pub(crate) mod typing;

use std::cell::OnceCell;

use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, Stmt};
use ruff_python_trivia::{has_leading_content, has_trailing_content, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::noqa;
use crate::project::doctest_code;
use crate::rules::comments::Comment;
use crate::rules::docstrings::PublicApi;
use crate::rules::docstrings::rst::{Target, collect_targets};
use crate::rules::layout::{ClassOrder, class_order};
use crate::rules::logic::KindsCache;
use crate::rules::module_variable::{ModuleVariable, module_variables};
use crate::rules::naming::ReportedNames;
use crate::syntax::{all_literal, code_string_values, is_name, last_segment};

// What the rules memoize for a file, each part built by the first rule that reads it.
#[derive(Default)]
pub(crate) struct FileState<'a> {
    public_api: OnceCell<PublicApi<'a>>,
    module_variables: OnceCell<Vec<ModuleVariable<'a>>>,
    rst_targets: OnceCell<Vec<Target<'a>>>,
    kinds: KindsCache<'a>,
    reported_names: ReportedNames<'a>,
    // The string literals of the `__all__` assignments, which renames of a name also rewrite.
    export_strings: OnceCell<Vec<&'a ast::ExprStringLiteral>>,
    string_words: OnceCell<rename::StringWords>,
    doctest_words: OnceCell<FxHashSet<String>>,
    class_orders: OnceCell<FxHashMap<TextRange, ClassOrder<'a>>>,
    // Every comment of the file with the directives it holds, which the comment rules read.
    comments: OnceCell<Vec<(TextRange, Comment)>>,
    source_spans: OnceCell<Vec<TextRange>>,
    file_noqa: OnceCell<bool>,
    debug_fields: OnceCell<edits::DebugFields<'a>>,
    // The `def`s that implement an `@overload` set, which Ruff's ANN401 leaves unchecked.
    overload_implementations: OnceCell<FxHashSet<TextSize>>,
    // The ends of the callees of the file's calls, which tell a reference that is called from one that is only read.
    call_ends: OnceCell<FxHashSet<TextSize>>,
}

impl<'a> FileState<'a> {
    // Whether the file has a file-level `noqa` directive, which a rewrite of its docstrings must leave alone.
    pub(crate) fn has_file_noqa(&self, checker: &Checker<'a>) -> bool {
        *self.file_noqa.get_or_init(|| noqa::has_file_noqa(checker.source, checker.comment_ranges()))
    }

    // The definitions of this file that belong to the library's public API, which the docstring rules share.
    pub(crate) fn public_api(&self, checker: &Checker<'a>) -> &PublicApi<'a> {
        checker.assert_flagged_cross_file();
        self.public_api.get_or_init(|| PublicApi::of(checker))
    }

    // The module-scope variables the case and prefix rules check, which the two of them share.
    pub(crate) fn module_variables(&self, checker: &Checker<'a>) -> &[ModuleVariable<'a>] {
        checker.assert_flagged_cross_file();
        self.module_variables.get_or_init(|| module_variables(checker))
    }

    // The parsed docstrings the reStructuredText rules check, empty unless `docstring-style = "rst"`.
    pub(crate) fn rst_targets(&self, checker: &Checker<'a>) -> &[Target<'a>] {
        self.rst_targets.get_or_init(|| collect_targets(checker))
    }

    // What `string-concatenation` reads off the module to tell the types of its operands.
    pub(crate) fn kinds(&self) -> &KindsCache<'a> {
        &self.kinds
    }

    // The names the single-letter and meaningless name rules reported so far.
    pub(crate) fn reported_names(&self) -> &ReportedNames<'a> {
        &self.reported_names
    }

    pub(crate) fn export_strings(&self, checker: &Checker<'a>) -> &[&'a ast::ExprStringLiteral] {
        self.export_strings.get_or_init(|| {
            checker
                .body()
                .iter()
                .filter_map(all_literal)
                .flat_map(|(_, elements)| elements)
                .filter_map(Expr::as_string_literal_expr)
                .collect()
        })
    }

    fn string_words(&self, checker: &Checker<'a>) -> &rename::StringWords {
        self.string_words
            .get_or_init(|| rename::StringWords::of(&code_string_values(checker.body()), self.export_strings(checker)))
    }

    // Every word of the `>>>` example lines of the file's docstrings, which a doctest names definitions by.
    pub(crate) fn doctest_words(&self, checker: &Checker<'a>) -> &FxHashSet<String> {
        self.doctest_words.get_or_init(|| {
            checker
                .string_values()
                .iter()
                .filter(|(_, text)| text.contains(">>>"))
                .flat_map(|(_, text)| doctest_code(text))
                .flat_map(|code| code.split(|char: char| !(char.is_alphanumeric() || char == '_')))
                .map(str::to_owned)
                .collect()
        })
    }

    // The order each class body must keep, which SMP201 and SMP203 both check against, so it is built once per class.
    pub(crate) fn class_orders(&self, checker: &Checker<'a>) -> &FxHashMap<TextRange, ClassOrder<'a>> {
        self.class_orders.get_or_init(|| {
            checker.class_defs().iter().map(|class| (class.range, class_order(checker, class))).collect()
        })
    }

    // The ranges of the t-strings, which keep their source text, so an edit inside one changes what they hold.
    pub(crate) fn source_text_spans(&self, checker: &Checker<'a>) -> &[TextRange] {
        self.source_spans.get_or_init(|| {
            let mut finder = SourceTextSpans::default();
            finder.visit_body(checker.body());
            finder.spans
        })
    }

    pub(crate) fn comments(&self, checker: &Checker) -> &[(TextRange, Comment)] {
        self.comments.get_or_init(|| {
            checker.comment_ranges().into_iter().map(|range| (range, Comment::at(checker.source, range))).collect()
        })
    }

    pub(crate) fn overload_implementations(&self, checker: &Checker<'a>) -> &FxHashSet<TextSize> {
        self.overload_implementations.get_or_init(|| typing::imprecise_annotation::overload_implementations(checker))
    }

    pub(crate) fn call_ends(&self, checker: &Checker<'a>) -> &FxHashSet<TextSize> {
        self.call_ends.get_or_init(|| {
            let mut ends = CallEnds::default();
            ends.visit_body(checker.body());
            ends.ends
        })
    }

    fn debug_fields(&self, checker: &Checker<'a>) -> &edits::DebugFields<'a> {
        self.debug_fields.get_or_init(|| {
            let mut finder = edits::DebugFields::default();
            finder.visit_body(checker.body());
            finder
        })
    }
}

// The ends of the callees of the calls in a file. A name or an attribute ends where its reference does, so a reference
// is the callee of a call when its end is one of these, wherever the call's `(` sits.
#[derive(Default)]
struct CallEnds {
    ends: FxHashSet<TextSize>,
}

impl<'a> Visitor<'a> for CallEnds {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.ends.insert(call.func.end());
        }
        visitor::walk_expr(self, expr);
    }
}

// A doctest runs in its module's globals, so an example that names a definition of the module uses it, whether or not
// the example imports it.
pub(crate) fn is_named_by_doctest(checker: &Checker, name: &str) -> bool {
    checker.state().doctest_words(checker).contains(name)
}

pub fn is_alone_on_lines(source: &str, range: TextRange) -> bool {
    !has_leading_content(range.start(), source) && !has_trailing_content(range.end(), source)
}

pub fn docstring(body: &[Stmt]) -> Option<&ast::ExprStringLiteral> {
    match body.first()? {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_string_literal_expr(),
        _ => None,
    }
}

// The leading whitespace of the line a node starts on; a node after a `;` shares the line, not its indentation.
pub fn indentation(source: &str, ranged: impl Ranged) -> &str {
    leading_indentation(source.full_line_str(ranged.start()))
}

// Whether the file is a Sphinx configuration, whose module variables Sphinx reads by their names and types.
pub fn is_sphinx_conf(checker: &Checker) -> bool {
    checker.file_name() == "conf.py"
}

pub fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

pub fn needs_parentheses(value: &Expr) -> bool {
    OperatorPrecedence::from_expr(value) < OperatorPrecedence::CallAttribute
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
}

pub fn expr_mentions(expr: &Expr, name: &str) -> bool {
    any_over_expr(expr, |expr| is_name(expr, name))
}

// The spans of the t-strings in `body`, which keep the text of their expressions, so a name changed inside one changes
// the output.
#[derive(Default)]
struct SourceTextSpans {
    spans: Vec<TextRange>,
}

impl<'a> Visitor<'a> for SourceTextSpans {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if matches!(expr, Expr::TString(_)) {
            self.spans.push(expr.range());
        }
        visitor::walk_expr(self, expr);
    }
}

// A module-level `if sys.version_info < ...: raise ...` or `sys.exit(...)` exists to run on interpreters older than the
// target and report them, so fixes must not add syntax or imports those
// interpreters lack.
pub fn has_version_gate(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_if_stmt).any(|gate| {
        any_over_expr(&gate.test, &|expr: &Expr| last_segment(expr) == Some("version_info"))
            && gate.body.last().is_some_and(|last| match last {
                Stmt::Raise(_) => true,
                Stmt::Expr(ast::StmtExpr { value, .. }) => {
                    value.as_call_expr().is_some_and(|call| last_segment(&call.func) == Some("exit"))
                }
                _ => false,
            })
    })
}

// The paragraphs of prose longer than `limit` words, each reported once from its first line to the line that passes
// the limit. Each item is a line's range and word count, or `None` where a line ends the paragraph.
pub(crate) fn long_paragraphs(
    lines: impl IntoIterator<Item = Option<(TextRange, usize)>>,
    limit: usize,
) -> Vec<TextRange> {
    let mut long = Vec::new();
    let mut words = 0;
    let mut first = None;
    for line in lines {
        let Some((range, count)) = line else {
            words = 0;
            first = None;
            continue;
        };
        let start = *first.get_or_insert(range.start());
        let before = words;
        words += count;
        if before <= limit && words > limit {
            long.push(TextRange::new(start, range.end()));
        }
    }
    long
}

// Ruff ranks names isort-style: constants in SCREAMING_SNAKE_CASE first, then CamelCase classes, then the rest. The two
// orders differ in the details: RUF022, sorting `__all__`, takes any name with a letter and no lowercase one for a
// constant and compares case-sensitively, while isort, sorting the names of a `from` import, wants more than one
// character for a constant and ignores case.
pub fn export_key(name: &str) -> (u8, String) {
    let rank = if name.chars().any(char::is_alphabetic) && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_owned())
}

pub fn isort_key(name: &str) -> (u8, String) {
    let rank = if name.len() > 1 && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_ascii_lowercase())
}

// `name = value` or `name: annotation = value`, binding a single plain name.
pub fn single_assignment(stmt: &Stmt) -> Option<(&ast::ExprName, Option<&Expr>, &Expr)> {
    let (target, annotation, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], None, value),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. }) => {
            (target.as_ref(), Some(annotation.as_ref()), value)
        }
        _ => return None,
    };
    Some((target.as_name_expr()?, annotation, value))
}
