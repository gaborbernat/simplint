use std::cell::LazyCell;

use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::computed_reads::ComputedReads;
use crate::rules::api::{definitions, names_are_followed, references_outside};
use crate::rules::is_named_by_doctest;
use crate::rules::public_api::Need;
use crate::semantic::Evaluation;
use crate::project::constructors::instance_attributes;
use crate::syntax::{Visibility, identifier_words, module_statements};

pub fn unused_definition(checker: &Checker) {
    nested_functions(checker);
    if !names_are_followed(checker) {
        return;
    }
    let project = checker.project();
    // Walks the module once, not once per definition.
    let local_reads = ComputedReads::of(checker.body());
    // Most definitions are ruled out before their name is looked up in the strings, so the words are split only then.
    let strings = LazyCell::new(|| string_words(checker));
    for definition in definitions(module_statements(checker.body())) {
        let stmt = definition.stmt;
        let name = definition.name.as_str();
        // A decorator may register the definition with something outside the run, so decorated ones are kept, and so
        // are public names a computed read may fetch.
        // An `obj.name` read may reach the definition through its module, imported under a name the index does not
        // follow, as `module = importlib.import_module(...)` binds one.
        let unused = !definition.decorated
            && !is_protocol_name(name)
            && !computed_read_hides(checker, name, &local_reads)
            && !checker.state().public_api(checker).covers(stmt.range(), Need::Any)
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
                let caller = if matches!(stmt, Stmt::ClassDef(_)) { "user" } else { "caller" };
                checker.report(
                    Rule::UnusedDefinition,
                    definition.name.range(),
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

// The identifiers each string of the module spells, with the ranges of the strings. A string annotation such as
// `cast("list[_Entry]", data)` names a definition the module's references do not show.
type StringWords<'a> = FxHashMap<&'a str, Vec<TextRange>>;

fn string_words<'c>(checker: &'c Checker) -> StringWords<'c> {
    let mut words: StringWords = FxHashMap::default();
    for (range, text) in checker.string_values() {
        for word in identifier_words(text).filter(|word| !word.is_empty()) {
            words.entry(word).or_default().push(*range);
        }
    }
    words
}

// Python calls a dunder such as `__repr__` itself, and a library calls a sunder such as Jupyter's `_repr_html_` or
// enum's `_missing_` by name, so neither needs a caller in the run.
fn is_protocol_name(name: &str) -> bool {
    Visibility::of(name) == Visibility::Dunder
        || (name.len() > 2 && name.starts_with('_') && name.ends_with('_') && !name.starts_with("__"))
}

// A computed read may fetch any name of the module it sits in, and any public name of the run, so a private name is
// judged by its own file, and a public one by the run. A read under a literal prefix reaches only the names that start
// with it, wherever they are defined.
fn computed_read_hides(checker: &Checker, name: &str, local_reads: &ComputedReads) -> bool {
    let run = checker.project().computed_reads();
    local_reads.reaches(name) || run.reaches_by_prefix(name) || (Visibility::of(name) == Visibility::Public && run.any)
}

// A framework calls these by name: Sphinx calls an extension's `setup`, and pytest calls its hooks and xunit-style
// `setup`/`teardown` functions.
fn is_framework_hook(name: &str) -> bool {
    matches!(name, "setup" | "teardown" | "setup_module" | "teardown_module") || name.starts_with("pytest_")
}

// A string outside the definition names it.
fn is_in_strings(strings: &StringWords, name: &str, definition: TextRange) -> bool {
    strings.get(name).is_some_and(|ranges| ranges.iter().any(|range| !definition.contains_range(*range)))
}

// hatchling loads the class that subclasses its hook interface from a configured custom hook file, which no import
// reaches. The index follows the bases of the class through the run's classes.
fn is_hatch_hook(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    checker.settings.project.hatch_hooks.iter().any(|hook| hook.as_path() == checker.path)
        && checker.project().is_hatch_hook_class(class.name.as_str())
}

// PYI046 and PYI049 report a private protocol or TypedDict class that Ruff finds unused in its file, so a class Ruff
// reads is kept. Ruff reads a load in a function or a deferred annotation, a string annotation, and the last binding of
// a name only, so a string outside the class's own docstrings and bare strings keeps the class here.
fn left_to_ruff(checker: &Checker, class: &ast::StmtClassDef, strings: &StringWords) -> bool {
    let semantic = checker.semantic();
    let protocol = class.bases().iter().any(|base| semantic.match_typing_expr(map_subscript(base), "Protocol"));
    let typed_dict = class.bases().iter().any(|base| semantic.match_typing_expr(base, "TypedDict"));
    let name = class.name.as_str();
    (protocol || typed_dict)
        && name.starts_with('_')
        && semantic.bindings(name).last().is_some_and(|binding| binding.range == class.name.range())
        && strings.get(name).is_none_or(|ranges| {
            // Walks the class body once for all the name's ranges.
            let bare = bare_strings(class);
            ranges.iter().all(|range| bare.iter().any(|statement| statement.contains_range(*range)))
        })
        && !semantic.references_to(name).iter().any(|reference| {
            reference.evaluation == Evaluation::Deferred
                || (reference.evaluation == Evaluation::Annotation && semantic.annotations_are_lazy())
        })
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

// A base class may supply the method a call reaches, and a decorator may register the class with something
// outside the run.
fn members(checker: &Checker, class: &ast::StmtClassDef, local_reads: &ComputedReads, strings: &StringWords) {
    let project = checker.project();
    // Walks the class body once, as each member's check reads the loads.
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
            && !checker.state().public_api(checker).covers(member.range(), Need::Any)
            && project.calls().attribute_loads(name) == 0
            && !computed_read_hides(checker, name, local_reads)
            && !project.is_attribute_assigned(name)
            && !project.is_referenced(name)
            && !references_outside(checker, name, member.range())
            && !is_read_in_class(&loads, name, member.range())
            && !is_in_strings(strings, name, member.range());
        if unused {
            let (verb, caller) = if is_method { ("called or read", "caller") } else { ("read", "reader") };
            let class_name = class.name.as_str();
            checker.report(
                Rule::UnusedDefinition,
                range,
                format!("`{class_name}.{name}` is never {verb} in the run; remove it or give it a {caller}"),
            );
        }
    }
    instance_attributes_of(checker, class, local_reads, strings);
}

// An attribute the methods store on the instance that nothing reads: no `obj.name` load, `getattr` literal or string
// in the run names it. A class that reads its instance as a whole, as `vars(self)`, pickling or a copy does, is kept
// whole, and so are the public attributes of a class the public API offers.
fn instance_attributes_of(
    checker: &Checker,
    class: &ast::StmtClassDef,
    local_reads: &ComputedReads,
    strings: &StringWords,
) {
    if reads_instance_whole(class) {
        return;
    }
    let project = checker.project();
    let public_class = checker.state().public_api(checker).covers(class.range(), Need::Any);
    let members: Vec<&str> = module_statements(&class.body)
        .filter_map(|member| match member {
            Stmt::FunctionDef(method) => Some(method.name.as_str()),
            Stmt::Assign(ast::StmtAssign { targets, .. }) => match targets.as_slice() {
                [Expr::Name(target)] => Some(target.id.as_str()),
                _ => None,
            },
            Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => target.as_name_expr().map(|name| name.id.as_str()),
            _ => None,
        })
        .collect();
    for (name, range) in instance_attributes(class) {
        let unread = !members.contains(&name)
            && !is_protocol_name(name)
            && !(public_class && Visibility::of(name) == Visibility::Public)
            && project.calls().attribute_loads(name) == 0
            && project.string_word_count(name) == 0
            && !computed_read_hides(checker, name, local_reads)
            && !project.is_referenced(name)
            && !is_in_strings(strings, name, TextRange::empty(range.start()));
        if unread {
            checker.report(
                Rule::UnusedDefinition,
                range,
                format!(
                    "`{}.{name}` is stored on the instance but never read in the run; remove it or give it a reader",
                    class.name
                ),
            );
        }
    }
}

// `vars(self)`, `self.__dict__`, and the hooks pickling and copying call read every attribute of the instance.
fn reads_instance_whole(class: &ast::StmtClassDef) -> bool {
    struct Whole(bool);
    impl<'a> Visitor<'a> for Whole {
        fn visit_expr(&mut self, expr: &'a Expr) {
            match expr {
                Expr::Attribute(attribute) if attribute.attr.as_str() == "__dict__" => self.0 = true,
                Expr::Name(name) if matches!(name.id.as_str(), "vars" | "asdict" | "astuple") => self.0 = true,
                _ => walk_expr(self, expr),
            }
        }
    }
    let hooks = ["__getstate__", "__reduce__", "__reduce_ex__", "__copy__", "__deepcopy__", "__slots__", "__getattr__"];
    if module_statements(&class.body).any(|member| match member {
        Stmt::FunctionDef(method) => hooks.contains(&method.name.as_str()),
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().any(|target| target.as_name_expr().is_some_and(|name| hooks.contains(&name.id.as_str())))
        }
        _ => false,
    }) {
        return true;
    }
    let mut whole = Whole(false);
    whole.visit_body(&class.body);
    whole.0
}

// A function defined inside another function that the enclosing function never reads: it is never called, returned or
// passed on. A `locals()` or `vars()` call, or a `global` or `nonlocal` statement naming it, can reach it by name.
fn nested_functions(checker: &Checker) {
    struct Outer<'c, 'a> {
        checker: &'c Checker<'a>,
    }
    impl<'a> Visitor<'a> for Outer<'_, 'a> {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if let Stmt::FunctionDef(function) = stmt {
                report_nested(self.checker, function);
            }
            walk_stmt(self, stmt);
        }
    }
    Outer { checker }.visit_body(checker.body());
}

fn report_nested(checker: &Checker, outer: &ast::StmtFunctionDef) {
    // A function that holds only definitions runs none of them; it is source text another tool reads, such as a
    // plugin a test writes out.
    let holds_only_definitions = outer.body.iter().all(|stmt| {
        matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::Pass(_))
            || stmt.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr())
    });
    let mut scope = Scope::default();
    scope.visit_body(&outer.body);
    if scope.dynamic || holds_only_definitions {
        return;
    }
    for inner in scope.functions {
        let name = inner.name.as_str();
        let read = scope.loads.get(name).is_some_and(|ranges| ranges.iter().any(|load| !inner.range().contains_range(*load)));
        if inner.decorator_list.is_empty() && !read && !scope.declared.contains(&name) && name != "_" {
            checker.report(
                Rule::UnusedDefinition,
                inner.name.range(),
                format!("`{}.{name}` is defined inside `{}` but never called or read; remove it", outer.name, outer.name),
            );
        }
    }
}

// The direct nested functions of a function body, the names it loads anywhere, nested scopes included, and whether it
// reaches its locals by name.
#[derive(Default)]
struct Scope<'a> {
    functions: Vec<&'a ast::StmtFunctionDef>,
    loads: FxHashMap<&'a str, Vec<TextRange>>,
    declared: Vec<&'a str>,
    dynamic: bool,
    depth: usize,
}

impl<'a> Visitor<'a> for Scope<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                if self.depth == 0 {
                    self.functions.push(function);
                }
                self.depth += 1;
                walk_stmt(self, stmt);
                self.depth -= 1;
            }
            Stmt::ClassDef(_) => {
                self.depth += 1;
                walk_stmt(self, stmt);
                self.depth -= 1;
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.declared.extend(names.iter().map(ast::Identifier::as_str));
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            if name.ctx.is_load() {
                if matches!(name.id.as_str(), "locals" | "vars" | "eval" | "exec") {
                    self.dynamic = true;
                }
                self.loads.entry(name.id.as_str()).or_default().push(name.range);
            } else if name.ctx.is_del() {
                self.loads.entry(name.id.as_str()).or_default().push(name.range);
            }
        }
        walk_expr(self, expr);
    }
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

// The class body names the member, as `x = property(_get_x)` or a dispatch table of its methods does.
fn is_read_in_class(loads: &FxHashMap<&str, Vec<TextRange>>, name: &str, definition: TextRange) -> bool {
    loads.get(name).is_some_and(|ranges| ranges.iter().any(|range| !definition.contains_range(*range)))
}
