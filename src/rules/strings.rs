use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Operator, PythonVersion, StringFlags};
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;

pub fn string_concatenation(checker: &mut Checker) {
    let mut finder = Concatenations { found: Vec::new() };
    finder.visit_body(checker.body());
    for (range, operands) in finder.found {
        let fix = fstring(checker, &operands).map(|text| Fix::unsafe_edit(Edit::range_replacement(text, range)));
        checker.report(Rule::StringConcatenation, range, "Build text with an f-string instead of `+`", fix);
    }
}

struct Concatenations<'a> {
    found: Vec<(TextRange, Vec<&'a Expr>)>,
}

impl<'a> Visitor<'a> for Concatenations<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::BinOp(ast::ExprBinOp { op: Operator::Add, .. }) = expr {
            let mut operands = Vec::new();
            flatten(expr, &mut operands);
            if operands.iter().any(|operand| matches!(operand, Expr::StringLiteral(_) | Expr::FString(_)))
                && !operands.iter().any(|operand| operand.is_bytes_literal_expr())
            {
                for operand in &operands {
                    self.visit_expr(operand);
                }
                self.found.push((expr.range(), operands));
                return;
            }
        }
        visitor::walk_expr(self, expr);
    }
}

fn flatten<'a>(expr: &'a Expr, operands: &mut Vec<&'a Expr>) {
    match expr {
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. }) => {
            flatten(left, operands);
            operands.push(right);
        }
        other => operands.push(other),
    }
}

fn fstring(checker: &Checker, operands: &[&Expr]) -> Option<String> {
    let mut quote = None;
    let mut content = String::new();
    let mut interpolated = false;
    let mut expressions = Vec::new();
    for operand in operands {
        match operand {
            Expr::StringLiteral(string) => {
                let literal = string.as_single_part_string()?;
                if literal.flags.is_triple_quoted()
                    || literal.flags.prefix().is_raw()
                    || literal.flags.prefix().is_unicode()
                {
                    return None;
                }
                check_quote(&mut quote, literal.flags.quote_style())?;
                content.push_str(&checker.text(literal.content_range()).replace('{', "{{").replace('}', "}}"));
            }
            Expr::FString(ast::ExprFString { value, .. }) => {
                let mut parts = value.iter();
                let (Some(ast::FStringPartRef::FString(fstring)), None) = (parts.next(), parts.next()) else {
                    return None;
                };
                if fstring.flags.is_triple_quoted() || fstring.flags.prefix().is_raw() {
                    return None;
                }
                check_quote(&mut quote, fstring.flags.quote_style())?;
                content.push_str(checker.text(TextRange::new(
                    fstring.start() + fstring.flags.opener_len(),
                    fstring.end() - fstring.flags.closer_len(),
                )));
                interpolated = true;
            }
            other => {
                let text = checker.text(other.range());
                if text.contains(['\n', '\\', '#', ':', '!', '{', '}']) {
                    return None;
                }
                expressions.push(text);
                content.push('{');
                content.push_str(text);
                content.push('}');
                interpolated = true;
            }
        }
    }
    let delimiter = quote.unwrap_or(checker.stylist.quote()).as_char();
    // Before Python 3.12 an f-string expression cannot reuse the string's own quote.
    if checker.settings.target_version < PythonVersion::PY312 && expressions.iter().any(|text| text.contains(delimiter))
    {
        return None;
    }
    let prefix = if interpolated { "f" } else { "" };
    let text = if interpolated { content } else { content.replace("{{", "{").replace("}}", "}") };
    Some(format!("{prefix}{delimiter}{text}{delimiter}"))
}

fn check_quote(quote: &mut Option<Quote>, style: Quote) -> Option<()> {
    match quote {
        Some(existing) if *existing != style => None,
        _ => {
            *quote = Some(style);
            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::StringConcatenation, "SMP112.py", PythonVersion::PY314 ; "string_concatenation")]
    #[test_case(Rule::StringConcatenation, "SMP112_old.py", PythonVersion::PY310 ; "string_concatenation_py310")]
    fn rule(rule: Rule, fixture: &str, target_version: PythonVersion) {
        check(rule, fixture, target_version);
    }
}
