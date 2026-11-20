use std::cell::OnceCell;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_trivia::has_leading_content;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::noqa;
use crate::project::doctest_code;
use crate::rules::comments::Comment;
use crate::rules::docstrings::rst::{Target, collect_targets};
use crate::rules::edits::DebugFields;
use crate::rules::layout::{ClassOrder, class_order};
use crate::rules::logic::{KindsCache, LocalScopes, RegexFacts};
use crate::rules::module_variable::{ModuleVariable, module_variables};
use crate::rules::naming::ReportedNames;
use crate::rules::public_api::PublicApi;
use crate::rules::{classes, rename, typing};
use crate::syntax::{Nesting, all_literal, code_string_values, identifier_words};

// Rules share per-file results to avoid walking the same syntax tree for each check.
#[derive(Default)]
pub(crate) struct FileState<'a> {
    public_api: OnceCell<PublicApi<'a>>,
    module_variables: OnceCell<Vec<ModuleVariable<'a>>>,
    rst_targets: OnceCell<Vec<Target<'a>>>,
    kinds: KindsCache<'a>,
    local_scopes: OnceCell<LocalScopes<'a>>,
    reported_names: ReportedNames<'a>,
    export_strings: OnceCell<Vec<&'a ast::ExprStringLiteral>>,
    string_words: OnceCell<rename::StringWords>,
    doctest_words: OnceCell<FxHashSet<String>>,
    class_orders: OnceCell<FxHashMap<TextRange, ClassOrder<'a>>>,
    comments: OnceCell<Vec<(TextRange, Comment)>>,
    source_spans: OnceCell<Vec<TextRange>>,
    file_noqa: OnceCell<bool>,
    debug_fields: OnceCell<DebugFields<'a>>,
    overload_implementations: OnceCell<FxHashSet<TextSize>>,
    call_ends: OnceCell<FxHashSet<TextSize>>,
    attribute_reads: OnceCell<FxHashMap<&'a str, Vec<&'a str>>>,
    scope_nesting: OnceCell<Nesting>,
    top_level_imports: OnceCell<FxHashSet<(&'a str, Option<&'a str>)>>,
    own_line_comments: OnceCell<Vec<TextRange>>,
    expected_to_raise: OnceCell<FxHashSet<&'a str>>,
    regex_facts: OnceCell<RegexFacts<'a>>,
}

impl<'a> FileState<'a> {
    pub(crate) fn has_file_noqa(&self, checker: &Checker<'a>) -> bool {
        *self.file_noqa.get_or_init(|| noqa::has_file_noqa(checker.source, checker.comment_ranges()))
    }

    pub(crate) fn public_api(&self, checker: &Checker<'a>) -> &PublicApi<'a> {
        checker.assert_flagged_cross_file();
        self.public_api.get_or_init(|| PublicApi::of(checker))
    }

    pub(crate) fn module_variables(&self, checker: &Checker<'a>) -> &[ModuleVariable<'a>] {
        checker.assert_flagged_cross_file();
        self.module_variables.get_or_init(|| module_variables(checker))
    }

    pub(crate) fn rst_targets(&self, checker: &Checker<'a>) -> &[Target<'a>] {
        self.rst_targets.get_or_init(|| collect_targets(checker))
    }

    pub(crate) fn kinds(&self) -> &KindsCache<'a> {
        &self.kinds
    }

    pub(crate) fn local_scopes(&self, checker: &Checker<'a>) -> &LocalScopes<'a> {
        self.local_scopes.get_or_init(|| LocalScopes::of(checker.body(), checker.function_defs()))
    }

    pub(crate) fn reported_names(&self) -> &ReportedNames<'a> {
        &self.reported_names
    }

    pub(crate) fn export_strings(&self, checker: &Checker<'a>) -> &[&'a ast::ExprStringLiteral] {
        self.export_strings.get_or_init(|| {
            checker
                .body()
                .iter()
                .filter_map(all_literal)
                .flat_map(|literal| literal.elements)
                .filter_map(Expr::as_string_literal_expr)
                .collect()
        })
    }

    pub(super) fn string_words(&self, checker: &Checker<'a>) -> &rename::StringWords {
        self.string_words
            .get_or_init(|| rename::StringWords::of(&code_string_values(checker.body()), self.export_strings(checker)))
    }

    // A `>>>` line in any string literal can refer to a module global without importing it.
    pub(crate) fn doctest_words(&self, checker: &Checker<'a>) -> &FxHashSet<String> {
        self.doctest_words.get_or_init(|| {
            checker
                .string_values()
                .iter()
                .filter(|(_, text)| text.contains(">>>"))
                .flat_map(|(_, text)| doctest_code(text))
                .flat_map(identifier_words)
                .map(str::to_owned)
                .collect()
        })
    }

    // SMP201 and SMP203 need the same order for their fixes to agree.
    pub(crate) fn class_orders(&self, checker: &Checker<'a>) -> &FxHashMap<TextRange, ClassOrder<'a>> {
        self.class_orders.get_or_init(|| {
            checker.class_defs().iter().map(|class| (class.range, class_order(checker, class))).collect()
        })
    }

    pub(crate) fn source_text_spans(&self, checker: &Checker<'a>) -> &[TextRange] {
        self.source_spans.get_or_init(|| {
            let mut finder = SourceTextSpans::default();
            finder.visit_body(checker.body());
            finder.spans
        })
    }

    pub(crate) fn in_source_text(&self, checker: &Checker<'a>, range: TextRange) -> bool {
        self.source_text_spans(checker).iter().any(|span| span.contains_range(range))
    }

    pub(crate) fn comments(&self, checker: &Checker) -> &[(TextRange, Comment)] {
        self.comments.get_or_init(|| {
            checker.comment_ranges().into_iter().map(|range| (range, Comment::at(checker.source, range))).collect()
        })
    }

    pub(crate) fn regex_facts(&self, checker: &Checker<'a>) -> &RegexFacts<'a> {
        self.regex_facts.get_or_init(|| RegexFacts::collect(checker))
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

    // Attribute reads are grouped by spelling across scopes and bindings.
    pub(crate) fn attribute_reads(&self, checker: &Checker<'a>) -> &FxHashMap<&'a str, Vec<&'a str>> {
        self.attribute_reads.get_or_init(|| {
            let mut reads = AttributeReads::default();
            reads.visit_body(checker.body());
            reads.attributes
        })
    }

    pub(crate) fn scope_nesting(&self, checker: &Checker<'a>) -> &Nesting {
        self.scope_nesting.get_or_init(|| {
            let scopes = checker
                .function_defs()
                .iter()
                .map(Ranged::range)
                .chain(checker.class_defs().iter().map(Ranged::range))
                .collect();
            Nesting::new(scopes)
        })
    }

    pub(crate) fn top_level_imports(&self, checker: &Checker<'a>) -> &FxHashSet<(&'a str, Option<&'a str>)> {
        self.top_level_imports.get_or_init(|| {
            let mut imports = FxHashSet::default();
            for stmt in checker.body() {
                match stmt {
                    Stmt::Import(ast::StmtImport { names, .. }) => imports.extend(
                        names.iter().filter(|alias| alias.asname.is_none()).map(|alias| (alias.name.as_str(), None)),
                    ),
                    Stmt::ImportFrom(ast::StmtImportFrom { module: Some(from), names, level: 0, .. }) => {
                        imports.extend(
                            names
                                .iter()
                                .filter(|alias| alias.asname.is_none())
                                .map(|alias| (from.as_str(), Some(alias.name.as_str()))),
                        );
                    }
                    _ => {}
                }
            }
            imports
        })
    }

    pub(super) fn debug_fields(&self, checker: &Checker<'a>) -> &DebugFields<'a> {
        self.debug_fields.get_or_init(|| DebugFields::of(checker.body()))
    }

    pub(crate) fn own_line_comments(&self, checker: &Checker) -> &[TextRange] {
        self.own_line_comments.get_or_init(|| {
            checker
                .comment_ranges()
                .iter()
                .copied()
                .filter(|range| !has_leading_content(range.start(), checker.source))
                .collect()
        })
    }

    pub(crate) fn expected_to_raise(&self, checker: &Checker<'a>) -> &FxHashSet<&'a str> {
        self.expected_to_raise.get_or_init(|| classes::expected_to_raise_classes(checker.body()))
    }
}

// A t-string exposes the source text of its expressions to code that reads the template.
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

// Matching the end of a reference to the end of a callee also recognizes calls with whitespace before the `(`.
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

#[derive(Default)]
struct AttributeReads<'a> {
    attributes: FxHashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Visitor<'a> for AttributeReads<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(attribute) = expr
            && let Expr::Name(base) = attribute.value.as_ref()
        {
            self.attributes.entry(base.id.as_str()).or_default().push(attribute.attr.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}
