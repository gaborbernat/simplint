use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, ensure};
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::linter::FixTable;
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::PythonVersion;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::fix::apply::{FixResult, fix_ruff_file};
use crate::fix::{add_fixes, errors};
use crate::lint::ruff::{Checked, RuffFile};

// Ruff's `lint_fix` loop as `ruff check --fix --unsafe-fixes` runs it, applying only the fixes of the rules simplint's
// fixes gave new findings. The other rules still report, as a suppression comment counts as unused when the rule
// it silences is off, and its removal would expose what it silenced, such as an import kept for others. Every pass
// is a `check`, which the caller remembers, so a pass over a source already checked costs nothing.
pub fn cleanup(
    path: &Path,
    source: &str,
    introduced: &FxHashSet<&'static str>,
    parser_version: PythonVersion,
    check: impl Fn(&str) -> Arc<Checked>,
) -> String {
    // Ruff sorts an import block only when no other statement shares its last line, while `ruff format` puts each
    // statement after a `;` on a line of its own, so the import lines are split as the formatter would split them.
    let start = if introduced.contains(RuffRule::UnsortedImports.name().as_str()) {
        split_import_lines(source, RuffFile::parse(source, parser_version).1.tokens())
    } else {
        source.to_owned()
    };
    lint_fix(path, &start, introduced, check).unwrap_or(start)
}

// Ruff's `lint_fix` loop, failing as it does when a fix breaks code that parsed.
fn lint_fix(
    path: &Path,
    start: &str,
    introduced: &FxHashSet<&'static str>,
    check: impl Fn(&str) -> Arc<Checked>,
) -> Result<String> {
    let mut checked = check(start);
    let valid = checked.parses && !checked.diagnostics.iter().any(Diagnostic::is_invalid_syntax);
    let mut transformed = start.to_owned();
    let mut fixed = FixTable::default();
    for iterations in 0.. {
        let offered = checked.diagnostics.iter().filter(|diagnostic| introduced.contains(diagnostic.name()));
        let converging = || !errors::ruff_failed_to_converge(path, iterations, &transformed, &checked.diagnostics);
        let Some(FixResult { code, fixes }) = fix_ruff_file(&transformed, offered).filter(|_| converging()) else {
            break;
        };
        add_fixes(&mut fixed, &fixes);
        transformed = code;
        checked = check(&transformed);
        ensure!(!valid || checked.parses, errors::ruff_broken_syntax(path, &transformed, &fixed));
    }
    Ok(transformed)
}

fn split_import_lines(source: &str, tokens: &Tokens) -> String {
    let mut split = String::with_capacity(source.len());
    let mut copied = TextSize::default();
    let mut statement = None::<&Token>;
    let mut tokens = tokens.iter().filter(|token| !token.kind().is_trivia()).peekable();
    while let Some(token) = tokens.next() {
        match token.kind() {
            TokenKind::Semi => {
                if let (Some(start), Some(next)) = (statement.take(), tokens.peek())
                    && matches!(start.kind(), TokenKind::Import | TokenKind::From)
                {
                    split.push_str(&source[TextRange::new(copied, token.start())]);
                    split.push('\n');
                    split.push_str(leading_indentation(source.full_line_str(start.start())));
                    copied = next.start();
                }
            }
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent => statement = None,
            _ => {
                statement.get_or_insert(token);
            }
        }
    }
    split.push_str(&source[copied.to_usize()..]);
    split
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use ruff_db::diagnostic::Diagnostic;
    use ruff_diagnostics::{Edit, Fix};
    use ruff_linter::registry::Rule as RuffRule;
    use ruff_linter::{IOError, Violation};
    use ruff_source_file::SourceFileBuilder;
    use ruff_text_size::{TextRange, TextSize};
    use rustc_hash::FxHashSet;
    use test_case::test_case;

    use super::lint_fix;
    use crate::lint::ruff::{Checked, Findings};

    // A check that offers one fix, inserting `inserted` at the start, on `start` alone, so the loop stops once it
    // applied.
    fn checker(start: &'static str, inserted: &'static str) -> impl Fn(&str) -> Arc<Checked> {
        move |source| {
            let mut diagnostics = Vec::new();
            if source == start {
                let file = SourceFileBuilder::new("module.py", source).finish();
                let mut diagnostic: Diagnostic =
                    IOError { message: String::new() }.into_diagnostic(TextRange::default(), &file);
                diagnostic.set_fix(Fix::safe_edit(Edit::insertion(inserted.to_owned(), TextSize::new(0))));
                diagnostics.push(diagnostic);
            }
            let parses = ruff_python_parser::parse_module(source).is_ok();
            Arc::new(Checked { diagnostics, parses, findings: Findings::default() })
        }
    }

    // A fix that breaks code that parsed is a bug in Ruff, so the cleanup gives up; code that did not parse is fixed on.
    #[test_case("x = 1\n", "(", None ; "breaking_valid_code_fails")]
    #[test_case("x = (\n", "y", Some("yx = (\n") ; "invalid_code_is_fixed_on")]
    fn lint_fix_fails_only_when_a_fix_breaks_valid_code(
        start: &'static str,
        inserted: &'static str,
        fixed: Option<&str>,
    ) {
        let introduced: FxHashSet<&'static str> = FxHashSet::from_iter([RuffRule::IOError.name().as_str()]);

        let result = lint_fix(Path::new("module.py"), start, &introduced, checker(start, inserted));

        assert_eq!(result.ok().as_deref(), fixed);
    }
}
