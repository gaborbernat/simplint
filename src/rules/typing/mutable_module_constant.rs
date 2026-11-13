use std::cell::OnceCell;

use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, CmpOp, Comprehension, Expr, ExprContext, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Import, ensure_import};
use crate::syntax::{last_segment, single_assignment};

use super::globals_writes::GlobalsWrites;

pub fn mutable_module_constant(checker: &Checker) {
    let project = checker.project();
    // A star import or a computed read hands the module's names to code this rule cannot follow.
    if project.is_star_imported() || project.is_computed_written() {
        return;
    }
    let globals_writes = OnceCell::new();
    let mut candidates = Vec::new();
    for stmt in checker.body() {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        let declared_final = annotation.is_some_and(|annotation| final_type(annotation).is_some());
        if !(is_constant_name(name) || declared_final) || is_dunder(name) {
            continue;
        }
        let Some(kind) = Kind::of(checker, value) else {
            continue;
        };
        // Another module may change the value through its import, which this module cannot see.
        if checker.semantic().is_rebound(name)
            || project.is_used_elsewhere(name)
            || project.is_read_opaquely(name)
            || project.is_module_written(name)
            || globals_writes.get_or_init(|| GlobalsWrites::of(checker)).writes(name)
        {
            continue;
        }
        candidates.push((target, annotation, value, kind));
    }
    if candidates.is_empty() {
        return;
    }
    let reads = Reads::of(checker, candidates.iter().map(|&(target, _, _, kind)| (target.id.as_str(), kind)).collect());
    for (target, annotation, value, kind) in candidates {
        let name = target.id.as_str();
        if reads.escaped.contains(name) {
            continue;
        }
        let immutable = kind.immutable(checker.target_version());
        checker.report_fix(
            Rule::MutableModuleConstant,
            target.range,
            format!("`{name}` is a {} that nothing changes; make it a `{immutable}`", kind.noun()),
            (!reads.typed.contains(name)).then(|| fix(checker, kind, value, annotation)).flatten(),
            format!("Make `{name}` a `{immutable}`"),
        );
    }
}

// An `UPPER_CASE` name, with any leading underscores, which reads as a constant.
fn is_constant_name(name: &str) -> bool {
    let bare = name.trim_start_matches('_');
    bare.chars().any(char::is_uppercase) && !bare.chars().any(char::is_lowercase)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    List,
    Set,
    Dict,
}

impl Kind {
    fn of(checker: &Checker, value: &Expr) -> Option<Self> {
        let semantic = checker.semantic();
        match value {
            Expr::List(_) | Expr::ListComp(_) => Some(Self::List),
            Expr::Set(_) | Expr::SetComp(_) => Some(Self::Set),
            Expr::Dict(_) | Expr::DictComp(_) => Some(Self::Dict),
            Expr::Call(ast::ExprCall { func, .. }) => [("list", Self::List), ("set", Self::Set), ("dict", Self::Dict)]
                .into_iter()
                .find(|(builtin, _)| semantic.is_builtin(func, builtin))
                .map(|(_, kind)| kind),
            _ => None,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Set => "set",
            Self::Dict => "dict",
        }
    }

    // Python 3.15 added the built-in `frozendict`; before it, a read-only view of the dict stands in.
    fn immutable(self, target: PythonVersion) -> &'static str {
        match self {
            Self::List => "tuple",
            Self::Set => "frozenset",
            Self::Dict if target >= PythonVersion::PY315 => "frozendict",
            Self::Dict => "MappingProxyType",
        }
    }
}

// The annotation inside `Final[...]`, or `Some(None)` for a bare `Final`; `None` when the annotation is not `Final`.
fn final_type(annotation: &Expr) -> Option<Option<&Expr>> {
    match annotation {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) if last_segment(value) == Some("Final") => {
            Some(Some(slice))
        }
        other if last_segment(other) == Some("Final") => Some(None),
        _ => None,
    }
}

// Code this module cannot see, such as a caller of a function that returns a slice of the value, may still rely on the
// list, set or dict API, so the fix is unsafe. An annotation the fix cannot follow, such as `typing.List[str]`, leaves it off.
fn fix(checker: &Checker, kind: Kind, value: &Expr, annotation: Option<&Expr>) -> Option<Fix> {
    let immutable = kind.immutable(checker.target_version());
    let mut edits = value_edits(checker, kind, value, immutable);
    let declared = annotation.and_then(|annotation| final_type(annotation).unwrap_or(Some(annotation)));
    if let Some(declared) = declared {
        edits.extend(annotation_edit(checker, kind, declared, immutable)?);
    }
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    let mut rest: Vec<Edit> = edits.collect();
    if immutable != "MappingProxyType" {
        return Some(Fix::applicable_edits(first, rest, Applicability::Unsafe));
    }
    match ensure_import(checker, "types", Some("MappingProxyType"), value.range())? {
        Import::Present => Some(Fix::applicable_edits(first, rest, Applicability::Unsafe)),
        Import::Add(import) => {
            rest.push(import);
            Some(Fix::applicable_edits(first, rest, Applicability::Unsafe).isolate(IsolationLevel::Group(1)))
        }
    }
}

fn value_edits(checker: &Checker, kind: Kind, value: &Expr, immutable: &str) -> Vec<Edit> {
    let range = value.range();
    let open = TextRange::at(range.start(), TextSize::from(1));
    let close = TextRange::at(range.end() - TextSize::from(1), TextSize::from(1));
    match (kind, value) {
        (Kind::List, Expr::List(ast::ExprList { elts, .. })) => {
            let single =
                elts.len() == 1 && !checker.text(range)[..usize::from(range.len()) - 1].trim_end().ends_with(',');
            vec![
                Edit::range_replacement("(".to_owned(), open),
                Edit::range_replacement(if single { ",)" } else { ")" }.to_owned(), close),
            ]
        }
        (Kind::List, Expr::ListComp(_)) => {
            vec![Edit::range_replacement("tuple(".to_owned(), open), Edit::range_replacement(")".to_owned(), close)]
        }
        (_, Expr::Call(ast::ExprCall { func, .. })) if kind != Kind::Dict || immutable == "frozendict" => {
            vec![Edit::range_replacement(immutable.to_owned(), func.range())]
        }
        _ => {
            vec![Edit::insertion(format!("{immutable}("), range.start()), Edit::insertion(")".to_owned(), range.end())]
        }
    }
}

// The edit that retypes a declared `list[T]`, `set[T]` or `dict[K, V]`, bare or subscripted, or `Some(None)` for an
// abstract annotation, such as `Sequence[str]`, that the immutable value already satisfies.
fn annotation_edit(checker: &Checker, kind: Kind, declared: &Expr, immutable: &str) -> Option<Option<Edit>> {
    let semantic = checker.semantic();
    let (base, slice) = match declared {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => (value.as_ref(), Some(slice.as_ref())),
        other => (other, None),
    };
    if !semantic.is_builtin(base, kind.noun()) {
        // `typing.List` and its kin name the mutable type too, which the fix cannot follow.
        let typing = ["List", "Set", "Dict"].iter().any(|alias| semantic.match_typing_expr(base, alias));
        return (!typing && !matches!(declared, Expr::StringLiteral(_))).then_some(None);
    }
    let text = match (kind, slice) {
        (Kind::List, Some(slice)) => format!("tuple[{}, ...]", checker.text(slice.range())),
        (_, Some(slice)) => format!("{immutable}[{}]", checker.text(slice.range())),
        (Kind::List, None) => "tuple".to_owned(),
        (_, None) => immutable.to_owned(),
    };
    Some(Some(Edit::range_replacement(text, declared.range())))
}

// Which of the names have a load in the module that does more than read the value. A read subscripts it, calls a method that leaves it as it
// is, iterates it, tests it, compares it, or passes it to a built-in that copies or reads it. Anything else, such as
// passing it to another function or returning it, may hand it to code that changes it.
//
// Some reads see the type itself, so the immutable value would change what they do: a list compares unequal to a tuple,
// `+` mixes them only by raising, a `tuple` has no `copy`, an operator on a `frozenset` or a mapping view returns another
// type than before, and `isinstance`, `type`, `repr`, `str` and `print` show the new type.
struct Reads<'a> {
    names: FxHashMap<&'a str, Kind>,
    reading: FxHashSet<TextRange>,
    escaped: FxHashSet<&'a str>,
    typed: FxHashSet<&'a str>,
}

impl<'a> Reads<'a> {
    fn of(checker: &Checker<'a>, names: FxHashMap<&'a str, Kind>) -> Self {
        let mut reads =
            Self { names, reading: FxHashSet::default(), escaped: FxHashSet::default(), typed: FxHashSet::default() };
        reads.visit_body(checker.body());
        reads
    }

    fn read(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr
            && self.names.contains_key(name.id.as_str())
        {
            self.reading.insert(name.range);
        }
    }

    // A read whose result depends on the type, for a constant of one of the kinds.
    fn typed_read(&mut self, expr: &'a Expr, kinds: &[Kind]) {
        self.read(expr);
        if let Expr::Name(name) = expr
            && self.names.get(name.id.as_str()).is_some_and(|kind| kinds.contains(kind))
        {
            self.typed.insert(name.id.as_str());
        }
    }
}

const KINDS: [Kind; 3] = [Kind::List, Kind::Set, Kind::Dict];

// Built-ins whose result shows the type of their argument.
const TYPE_BUILTINS: [&str; 5] = ["isinstance", "type", "repr", "str", "print"];

// Methods that read a list, set or dict without changing it.
const READ_METHODS: [&str; 12] = [
    "get",
    "keys",
    "values",
    "items",
    "index",
    "count",
    "union",
    "intersection",
    "difference",
    "symmetric_difference",
    "issubset",
    "issuperset",
];

// Built-ins that read or copy their arguments.
const READING_BUILTINS: [&str; 18] = [
    "len",
    "sorted",
    "list",
    "tuple",
    "set",
    "frozenset",
    "dict",
    "iter",
    "enumerate",
    "zip",
    "map",
    "filter",
    "any",
    "all",
    "sum",
    "min",
    "max",
    "reversed",
];

// Methods that read their argument without keeping it.
const READING_METHODS: [&str; 10] = [
    "join",
    "extend",
    "update",
    "union",
    "intersection",
    "difference",
    "issubset",
    "issuperset",
    "startswith",
    "endswith",
];

impl<'a> Visitor<'a> for Reads<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::For(ast::StmtFor { iter, .. }) => self.read(iter),
            Stmt::If(ast::StmtIf { test, elif_else_clauses, .. }) => {
                self.read(test);
                for clause in elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.read(test);
                    }
                }
            }
            Stmt::While(ast::StmtWhile { test, .. }) | Stmt::Assert(ast::StmtAssert { test, .. }) => self.read(test),
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_comprehension(&mut self, comprehension: &'a Comprehension) {
        self.read(&comprehension.iter);
        for test in &comprehension.ifs {
            self.read(test);
        }
        visitor::walk_comprehension(self, comprehension);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx, range, .. }) if self.names.contains_key(id.as_str()) => {
                // The one store is the definition, as the name is not rebound.
                if matches!(ctx, ExprContext::Del) || matches!(ctx, ExprContext::Load) && !self.reading.contains(range)
                {
                    self.escaped.insert(id.as_str());
                }
            }
            Expr::Subscript(ast::ExprSubscript { value, ctx: ExprContext::Load, .. }) => self.read(value),
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Load, .. }) if attr == "copy" => {
                self.typed_read(value, &[Kind::List]);
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Load, .. })
                if READ_METHODS.contains(&attr.as_str()) =>
            {
                self.read(value);
            }
            Expr::Compare(compare) => {
                // Only the container of a membership test is not compared as a whole.
                let last = compare.ops.len();
                for (index, operand) in compare.operands.iter().enumerate() {
                    if index == last && matches!(compare.ops[last - 1], CmpOp::In | CmpOp::NotIn) {
                        self.read(operand);
                    } else {
                        self.typed_read(operand, &[Kind::List]);
                    }
                }
            }
            Expr::BinOp(ast::ExprBinOp { left, right, .. }) => {
                self.typed_read(left, &KINDS);
                self.typed_read(right, &KINDS);
            }
            Expr::BoolOp(ast::ExprBoolOp { values, .. }) => values.iter().for_each(|value| self.read(value)),
            Expr::UnaryOp(ast::ExprUnaryOp { operand, .. }) => self.read(operand),
            Expr::If(ast::ExprIf { test, .. }) => self.read(test),
            Expr::Starred(ast::ExprStarred { value, .. }) => self.read(value),
            Expr::Dict(ast::ExprDict { items, .. }) => {
                items.iter().filter(|item| item.key.is_none()).for_each(|item| self.read(&item.value));
            }
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if func.as_name_expr().is_some_and(|builtin| TYPE_BUILTINS.contains(&builtin.id.as_str())) =>
            {
                arguments.args.iter().for_each(|argument| self.typed_read(argument, &KINDS));
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                let reads_arguments = match func.as_ref() {
                    Expr::Name(builtin) => READING_BUILTINS.contains(&builtin.id.as_str()),
                    Expr::Attribute(attribute) => READING_METHODS.contains(&attribute.attr.as_str()),
                    _ => false,
                };
                if reads_arguments {
                    arguments.args.iter().for_each(|argument| self.read(argument));
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
