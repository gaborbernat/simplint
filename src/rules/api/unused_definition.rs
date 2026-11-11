use std::cell::LazyCell;

use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::computed_reads::ComputedReads;
use crate::rules::api::references_outside;
use crate::rules::docstrings::Need;
use crate::rules::is_named_by_doctest;
use crate::semantic::Evaluation;
use crate::syntax::{Visibility, module_statements};

pub fn unused_definition(checker: &Checker) {
    let project = checker.project();
    // An `import *` of this module hands its names to code the index cannot follow, as `unused-export` assumes too.
    if checker.is_test_module() || project.is_star_import_source() {
        return;
    }
    let api = checker.state().public_api(checker);
    let local_reads = ComputedReads::of(checker.body());
    // Most definitions are ruled out before their name is looked up in the strings, so the words are split only then.
    let strings = LazyCell::new(|| string_words(checker));
    for stmt in module_statements(checker.body()) {
        let (name, range, decorated, caller) = match stmt {
            Stmt::FunctionDef(function) => {
                (function.name.as_str(), function.name.range(), !function.decorator_list.is_empty(), "caller")
            }
            Stmt::ClassDef(class) => {
                (class.name.as_str(), class.name.range(), !class.decorator_list.is_empty(), "user")
            }
            _ => continue,
        };
        // A decorator may register the definition with something outside the run, so decorated ones are kept, and so are
        // public names a computed read may fetch.
        // An `obj.name` read may reach the definition through its module, imported under a name the index does not
        // follow, as `module = importlib.import_module(...)` binds one.
        let unused = !decorated
            && !is_protocol_name(name)
            && !computed_read_hides(checker, name, &local_reads)
            && !api.covers(stmt.range(), Need::Any)
            && !checker.dunder_all().lists(name)
            && !is_framework_hook(name)
            && !project.is_used_elsewhere(name)
            && !project.is_base_class(name)
            && project.calls().attribute_loads(name) == 0
            && !references_outside(checker, name, stmt.range())
            && !is_in_strings(&strings, name, stmt.range())
            && !is_named_by_doctest(checker, name)
            && !matches!(stmt, Stmt::ClassDef(class) if is_hatch_hook(checker, class));
        if unused {
            if !matches!(stmt, Stmt::ClassDef(class) if left_to_ruff(checker, class, &strings)) {
                checker.report(
                    Rule::UnusedDefinition,
                    range,
                    format!("`{name}` is never used in the run; remove it or give it a {caller}"),
                );
            }
        } else if let Stmt::ClassDef(class) = stmt
            && class.bases().is_empty()
            && class.keywords().is_empty()
            && class.decorator_list.is_empty()
        {
            // The members of an unused class are reported with it, so only the members of a used class are judged.
            members(checker, class, &local_reads, &strings);
        }
    }
}

// PYI046 and PYI049 report a private protocol or TypedDict class that Ruff finds unused in its file, so a class Ruff
// reads is kept. Ruff reads a load in a function or a deferred annotation, a string annotation, and the last binding of
// a name only, so a string outside the class's own docstrings and bare strings keeps the class here.
fn left_to_ruff(checker: &Checker, class: &ast::StmtClassDef, strings: &StringWords) -> bool {
    let name = class.name.as_str();
    let from_ruff = &checker.settings.from_ruff;
    let semantic = checker.semantic();
    let protocol = from_ruff.ruff_runs(RuffRule::UnusedPrivateProtocol)
        && class.bases().iter().any(|base| semantic.match_typing_expr(map_subscript(base), "Protocol"));
    let typed_dict = from_ruff.ruff_runs(RuffRule::UnusedPrivateTypedDict)
        && class.bases().iter().any(|base| semantic.match_typing_expr(base, "TypedDict"));
    let lazy = semantic.annotations_are_lazy();
    (protocol || typed_dict)
        && name.starts_with('_')
        && semantic.bindings(name).last().is_some_and(|binding| binding.range == class.name.range())
        && strings.get(name).is_none_or(|ranges| {
            let bare = bare_strings(class);
            ranges.iter().all(|range| bare.iter().any(|statement| statement.contains_range(*range)))
        })
        && !semantic.references_to(name).iter().any(|reference| {
            reference.evaluation == Evaluation::Deferred || (reference.evaluation == Evaluation::Annotation && lazy)
        })
}

// hatchling loads the class that subclasses its hook interface from a configured custom hook file, which no import
// reaches. The index follows the bases of the class through the run's classes.
fn is_hatch_hook(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    checker.settings.project.hatch_hooks.iter().any(|hook| hook.as_path() == checker.path)
        && checker.project().is_hatch_hook_class(class.name.as_str())
}

// The strings a class's body and its functions state on their own, such as docstrings, which Ruff does not read.
fn bare_strings(class: &ast::StmtClassDef) -> Vec<TextRange> {
    struct Bare(Vec<TextRange>);
    impl<'a> Visitor<'a> for Bare {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt
                && matches!(value.as_ref(), Expr::StringLiteral(_) | Expr::FString(_))
            {
                self.0.push(value.range());
            }
            walk_stmt(self, stmt);
        }
    }
    let mut bare = Bare(Vec::new());
    for stmt in &class.body {
        bare.visit_stmt(stmt);
    }
    bare.0
}

// The methods and plain attributes of a class with no base or decorator. A base class may supply the method a call
// reaches, and a decorator may register the class with something outside the run.
fn members(checker: &Checker, class: &ast::StmtClassDef, local_reads: &ComputedReads, strings: &StringWords) {
    let project = checker.project();
    let api = checker.state().public_api(checker);
    let calls = project.calls();
    let class_name = class.name.as_str();
    let loads = class_loads(class);
    for member in module_statements(&class.body) {
        let (name, range, is_method, decorated) = match member {
            Stmt::FunctionDef(method) => {
                (method.name.as_str(), method.name.range(), true, !method.decorator_list.is_empty())
            }
            Stmt::Assign(ast::StmtAssign { targets, .. }) => match targets.as_slice() {
                [Expr::Name(target)] => (target.id.as_str(), target.range(), false, false),
                _ => continue,
            },
            _ => continue,
        };
        // An attribute store may be read through an object the run does not type, so an assigned name is kept.
        let unused = !decorated
            && !is_protocol_name(name)
            && !api.covers(member.range(), Need::Any)
            && calls.attribute_loads(name) == 0
            && !computed_read_hides(checker, name, local_reads)
            && !project.is_attribute_assigned(name)
            && !project.is_referenced(name)
            && !references_outside(checker, name, member.range())
            && !is_read_in_class(&loads, name, member.range())
            && !is_in_strings(strings, name, member.range());
        if unused {
            let (verb, caller) = if is_method { ("called or read", "caller") } else { ("read", "reader") };
            checker.report(
                Rule::UnusedDefinition,
                range,
                format!("`{class_name}.{name}` is never {verb} in the run; remove it or give it a {caller}"),
            );
        }
    }
}

// A computed read may fetch any name of the module it sits in, and any public name of the run, so a private name is
// judged by its own file, and a public one by the run. A read under a literal prefix reaches only the names that start
// with it, wherever they are defined.
fn computed_read_hides(checker: &Checker, name: &str, local_reads: &ComputedReads) -> bool {
    let run = checker.project().computed_reads();
    local_reads.reaches(name) || run.reaches_by_prefix(name) || (Visibility::of(name) == Visibility::Public && run.any)
}

// The class body names the member, as `x = property(_get_x)` or a dispatch table of its methods does.
fn is_read_in_class(loads: &FxHashMap<&str, Vec<TextRange>>, name: &str, definition: TextRange) -> bool {
    loads.get(name).is_some_and(|ranges| ranges.iter().any(|range| !definition.contains_range(*range)))
}

// The names the class body loads, with the ranges of the loads, so each member finds its reads in one walk.
fn class_loads(class: &ast::StmtClassDef) -> FxHashMap<&str, Vec<TextRange>> {
    struct Loads<'a>(FxHashMap<&'a str, Vec<TextRange>>);

    impl<'a> Visitor<'a> for Loads<'a> {
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Name(read) = expr
                && read.ctx.is_load()
            {
                self.0.entry(read.id.as_str()).or_default().push(read.range);
            }
            walk_expr(self, expr);
        }
    }

    let mut loads = Loads(FxHashMap::default());
    loads.visit_body(&class.body);
    loads.0
}

// The identifiers each string of the module spells, with the ranges of the strings: a string annotation such as
// `cast("list[_Entry]", data)` names a definition the module's references do not show.
type StringWords<'a> = FxHashMap<&'a str, Vec<TextRange>>;

fn string_words<'c>(checker: &'c Checker) -> StringWords<'c> {
    let mut words: StringWords = FxHashMap::default();
    for (range, text) in checker.string_values() {
        for word in text.split(|char: char| !(char.is_alphanumeric() || char == '_')).filter(|word| !word.is_empty()) {
            words.entry(word).or_default().push(*range);
        }
    }
    words
}

// A string outside the definition names it.
fn is_in_strings(strings: &StringWords, name: &str, definition: TextRange) -> bool {
    strings.get(name).is_some_and(|ranges| ranges.iter().any(|range| !definition.contains_range(*range)))
}

// Python calls a dunder such as `__repr__` itself, and a library calls a sunder such as Jupyter's `_repr_html_` or enum's
// `_missing_` by name, so neither needs a caller in the run.
fn is_protocol_name(name: &str) -> bool {
    Visibility::of(name) == Visibility::Dunder
        || (name.len() > 2 && name.starts_with('_') && name.ends_with('_') && !name.starts_with("__"))
}

// A framework calls these by name: Sphinx calls an extension's `setup`, and pytest calls its hooks and xunit-style
// `setup`/`teardown` functions.
fn is_framework_hook(name: &str) -> bool {
    matches!(name, "setup" | "teardown" | "setup_module" | "teardown_module") || name.starts_with("pytest_")
}
