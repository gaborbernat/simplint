use std::hash::Hasher;
use std::path::Path;

use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{self as ast, ModModule, Stmt};
use ruff_python_parser::{Parsed, parse_module};
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use crate::syntax::for_each_body;

// A statement `duplicate-code` compares, hashed from its tokens, with the body it sits in, its range and its line.
#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct Statement {
    pub(crate) hash: u64,
    pub(crate) body: u32,
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) line: u32,
}

// The statements of every body that `duplicate-code` compares, each with the hash of its tokens. A statement's tokens
// are those of the statements nested in it too, so each token is hashed once and a statement's hash is taken from the
// running polynomial hash of the file's tokens, rather than hashing a deeply nested token once per statement around it.
pub(super) fn statements(parsed: &Parsed<ModModule>, source: &str) -> Vec<Statement> {
    // Any odd multiplier works modulo 2^64; the windows that hash alike are compared by their texts after.
    const BASE: u64 = 0x9E37_79B9_7F4A_7C15;
    let lines = LineIndex::from_source_text(source);
    let tokens: Vec<(TextSize, u64)> = compared_tokens(parsed.tokens())
        .map(|token| {
            let text = &source[token.range()];
            let mut hasher = FxHasher::default();
            hasher.write_usize(text.len());
            hasher.write(text.as_bytes());
            (token.start(), hasher.finish())
        })
        .collect();
    let mut prefix = Vec::with_capacity(tokens.len() + 1);
    let mut powers = Vec::with_capacity(tokens.len() + 1);
    let (mut hash, mut power) = (0u64, 1u64);
    for (_, token) in &tokens {
        prefix.push(hash);
        powers.push(power);
        hash = hash.wrapping_mul(BASE).wrapping_add(*token);
        power = power.wrapping_mul(BASE);
    }
    prefix.push(hash);
    powers.push(power);
    let mut statements = Vec::new();
    compared_statements(&parsed.syntax().body, &mut |body, stmt| {
        let first = tokens.partition_point(|(start, _)| *start < stmt.start());
        let end = tokens.partition_point(|(start, _)| *start < stmt.end());
        let hash = prefix[end].wrapping_sub(prefix[first].wrapping_mul(powers[end - first]));
        statements.push(Statement {
            hash,
            body,
            start: stmt.start().to_u32(),
            end: stmt.end().to_u32(),
            line: u32::try_from(lines.line_index(stmt.start()).get())
                .expect("a file has fewer lines than a u32 counts"),
        });
    });
    statements
}

// The tokens of each statement of the file at `path` that `wanted` holds by its position among the compared
// statements, written one after the other with their lengths, so two statements match when their texts are equal. None
// when the file no longer holds the statements `expected` records.
pub(crate) fn statement_texts(
    path: &Path,
    wanted: &FxHashSet<usize>,
    expected: &[Statement],
) -> Option<FxHashMap<usize, Vec<u8>>> {
    let source = std::fs::read_to_string(path).ok()?;
    let parsed = parse_module(&source).ok()?;
    let mut texts = FxHashMap::default();
    let mut at = 0;
    let mut same = true;
    compared_statements(&parsed.syntax().body, &mut |_, stmt| {
        same &= expected.get(at).is_some_and(|known| known.start == stmt.start().to_u32());
        if same && wanted.contains(&at) {
            let mut text = Vec::new();
            for token in statement_tokens(&parsed, &source, stmt.range()) {
                text.extend_from_slice(&token.len().to_le_bytes());
                text.extend_from_slice(token.as_bytes());
            }
            texts.insert(at, text);
        }
        at += 1;
    });
    (same && at == expected.len()).then_some(texts)
}

// Each statement `duplicate-code` compares, with the number of the body it sits in. Imports and docstrings are left
// out, as pylint's duplicate check leaves them out.
fn compared_statements<'a>(body: &'a [Stmt], visit: &mut impl FnMut(u32, &'a Stmt)) {
    let mut bodies = 0;
    for_each_body(body, &mut |statements, _| {
        let compared = statements.iter().filter(|stmt| match stmt {
            Stmt::Import(_) | Stmt::ImportFrom(_) => false,
            Stmt::Expr(ast::StmtExpr { value, .. }) => !value.is_string_literal_expr(),
            _ => true,
        });
        for stmt in compared {
            visit(bodies, stmt);
        }
        bodies += 1;
    });
}

// The text of a statement's tokens, without the layout and comment tokens, so layout and comments do not count.
fn statement_tokens<'s>(
    parsed: &'s Parsed<ModModule>,
    source: &'s str,
    range: TextRange,
) -> impl Iterator<Item = &'s str> {
    compared_tokens(parsed.tokens().in_range(range)).map(|token| &source[token.range()])
}

// The tokens that make up a statement's text: those other than layout and comments.
fn compared_tokens(tokens: &[Token]) -> impl Iterator<Item = &Token> {
    tokens.iter().filter(|token| {
        !matches!(
            token.kind(),
            TokenKind::Comment
                | TokenKind::NonLogicalNewline
                | TokenKind::Newline
                | TokenKind::Indent
                | TokenKind::Dedent
        )
    })
}
