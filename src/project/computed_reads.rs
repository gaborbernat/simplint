use std::collections::BTreeSet;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::syntax::{ExprCollector, attribute_call};

// A body reads names the program computes when it runs. `getattr(obj, name)`, `vars`, `dir` and `__dict__` may reach
// any attribute, while a literal start such as `getattr(obj, "visit_" + kind)` reaches only the names with that prefix,
// as a dispatch table does.
#[derive(Clone, Default, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct ComputedReads {
    pub(crate) any: bool,
    prefixes: BTreeSet<String>,
}

pub(crate) enum Reach {
    Any,
    Prefix(String),
}

impl ComputedReads {
    pub(crate) fn of(body: &[Stmt]) -> Self {
        let mut computed = ExprCollector::new(Self::read);
        computed.visit_body(body);
        let mut reads = Self::default();
        for read in computed.found {
            reads.insert(read);
        }
        reads
    }

    pub(crate) fn read(expr: &Expr) -> Option<Reach> {
        if let Some(attribute) = expr.as_attribute_expr() {
            return (attribute.attr.as_str() == "__dict__").then_some(Reach::Any);
        }
        if let Some((_, name)) = attribute_call(expr, &["getattr"]) {
            return (!name.is_string_literal_expr())
                .then(|| Self::literal_prefix(name).map_or(Reach::Any, Reach::Prefix));
        }
        let ast::ExprCall { func, .. } = expr.as_call_expr()?;
        match func.as_name_expr()?.id.as_str() {
            "vars" | "dir" => Some(Reach::Any),
            _ => None,
        }
    }

    // Underscores alone pick out no family of names, so `"_" + name` reads like any name.
    fn literal_prefix(name: &Expr) -> Option<String> {
        let prefix = match name {
            Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::Add, .. }) => {
                left.as_string_literal_expr()?.value.to_str()
            }
            Expr::FString(fstring) => match fstring.value.elements().next()? {
                ast::InterpolatedStringElement::Literal(literal) => &literal.value,
                ast::InterpolatedStringElement::Interpolation(_) => return None,
            },
            _ => return None,
        };
        prefix.contains(|char: char| char != '_').then(|| prefix.to_owned())
    }

    pub(crate) fn insert(&mut self, read: Reach) {
        match read {
            Reach::Any => self.any = true,
            Reach::Prefix(prefix) => _ = self.prefixes.insert(prefix),
        }
    }

    pub(crate) fn extend(&mut self, other: &Self) {
        self.any |= other.any;
        self.prefixes.extend(other.prefixes.iter().cloned());
    }

    pub(crate) fn reaches(&self, name: &str) -> bool {
        self.any || self.reaches_by_prefix(name)
    }

    pub(crate) fn reaches_by_prefix(&self, name: &str) -> bool {
        self.prefixes.iter().any(|prefix| name.starts_with(prefix.as_str()))
    }
}

impl CacheKey for ComputedReads {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.any.cache_key(state);
        self.prefixes.iter().cloned().collect::<Vec<_>>().cache_key(state);
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_parser::parse_module;
    use test_case::test_case;

    use super::ComputedReads;

    #[test_case("getattr(obj, kind + \"_suffix\")" ; "computed_left_operand")]
    #[test_case("getattr(obj, f\"\")" ; "empty_fstring")]
    fn name_without_a_literal_start_reaches_any_name(source: &str) {
        let parsed = parse_module(source).expect("parses");
        let reads = ComputedReads::of(&parsed.syntax().body);
        assert!(reads.any && reads.prefixes.is_empty());
    }
}
