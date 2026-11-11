use ruff_python_ast::{Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::syntax::all_literal;

// The one-name-per-line layout `ruff format` gives a list with a trailing comma, so the fix needs no formatter pass.
pub(super) fn exploded(checker: &Checker, base: &str, items: &[String]) -> String {
    let indent = checker.stylist.indentation().as_str();
    let eol = checker.line_ending();
    let lines = items.iter().fold(String::new(), |text, item| text + base + indent + item + "," + eol);
    format!("[{eol}{lines}{base}]")
}

// The elements of an `__all__` list or tuple with the statement binding it, which bounds where a parenthesized element
// ends.
pub(super) struct AllList<'a> {
    stmt: &'a Stmt,
    value: TextRange,
    elements: &'a [Expr],
}

impl<'a> AllList<'a> {
    pub(super) fn of(stmt: &'a Stmt) -> Option<Self> {
        let literal = all_literal(stmt)?;
        Some(Self { stmt, value: literal.value, elements: literal.elements })
    }

    pub(super) fn new(stmt: &'a Stmt, value: TextRange, elements: &'a [Expr]) -> Self {
        Self { stmt, value, elements }
    }

    pub(super) fn value(&self) -> TextRange {
        self.value
    }

    pub(super) fn elements(&self) -> &'a [Expr] {
        self.elements
    }

    pub(super) fn brackets(&self, checker: &Checker) -> (&'static str, &'static str) {
        if checker.source[self.value].starts_with('[') { ("[", "]") } else { ("(", ")") }
    }

    pub(super) fn items(&self, checker: &Checker) -> Vec<String> {
        self.elements.iter().map(|elt| checker.text(elt.range()).to_owned()).collect()
    }

    pub(super) fn has_comment(&self, checker: &Checker) -> bool {
        checker.has_comment_in(self.value)
    }

    pub(super) fn one_per_line(&self, checker: &Checker) -> bool {
        self.elements.iter().all(|elt| is_alone_on_line(checker, elt.range()))
    }

    pub(super) fn element_range(&self, checker: &Checker, element: &Expr) -> TextRange {
        checker.parenthesized_range(element, self.stmt.into())
    }
}

// Whether nothing but indentation comes before the range on its first line.
pub(super) fn is_alone_on_line(checker: &Checker, range: TextRange) -> bool {
    checker.text(TextRange::new(checker.source.line_start(range.start()), range.start())).trim().is_empty()
}
