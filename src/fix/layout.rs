use ruff_python_ast::Operator;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_trivia::{SuppressionKind as FormatSuppression, has_leading_content, is_pragma_comment};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

// Lines too long for any layout `ruff format` can give the code, and pragma comments it would move off their code,
// judged from the tokens alone: the formatter decides the layout after simplint, so a fix is judged by what every
// layout of its result shows, and the same code gets the same answer however it is laid out now. The formatter
// never splits a name, a string or an f-string, nor a name from the bracket opening after it. It wraps a value it
// cannot fit in parentheses one indentation level deeper only if it then fits, puts each item of a bracket it splits
// on a line of its own, keeps a dict value or keyword argument after its key, and leaves a statement's start and a
// `def`, `class` or import name where they are. Code under `# fmt: off` or `# fmt: skip` keeps the layout it has.
pub struct Layout {
    pub overlong: usize,
    pub moved_pragmas: usize,
}

// The widths the layout is judged against: Ruff's line length limit, and the formatter's line and indent widths.
#[derive(Clone, Copy)]
pub struct Widths {
    pub limit: usize,
    pub line: usize,
    pub indent: usize,
}

// A token outside trivia, with an f-string as one token, and where it sits in the code's structure.
struct Placed {
    kind: TokenKind,
    range: TextRange,
    // Brackets open before it, and indented blocks around its statement.
    depth: usize,
    level: usize,
    // Where its logical line, and its item in the innermost open bracket, start.
    line: TextSize,
    item: Option<TextSize>,
}

impl Layout {
    pub fn of(source: &str, tokens: &Tokens, widths: Widths, formatted: bool) -> Self {
        let mut walk = Walk {
            source,
            widths,
            placed: Vec::new(),
            moved_pragmas: 0,
            overlong_comments: 0,
            pragmas: 0,
            comments: Vec::new(),
            frozen: Frozen { whole: !formatted, ..Frozen::default() },
            depth: 0,
            level: 0,
            line: None,
            items: Vec::new(),
            fstrings: 0,
            fstring_start: TextSize::default(),
        };
        for token in tokens {
            walk.place(token);
        }
        let Walk { placed, moved_pragmas, overlong_comments, frozen, .. } = walk;
        let frozen = frozen.finish(source);
        let overlong = (0..placed.len())
            .filter(|&index| !frozen.contains(placed[index].range.start()) && overlong(source, &placed, index, widths))
            .count()
            + frozen.overlong_lines(source, widths.limit)
            + overlong_comments;
        Self { overlong, moved_pragmas }
    }
}

// One pass over the tokens: what it placed and found so far, and where in the code's structure it stands.
struct Walk<'s> {
    source: &'s str,
    widths: Widths,
    placed: Vec<Placed>,
    moved_pragmas: usize,
    overlong_comments: usize,
    // How many comments of the logical line so far are pragmas, and those comments.
    pragmas: usize,
    comments: Vec<TextRange>,
    frozen: Frozen,
    // Brackets open, indented blocks around the statement, and where its logical line starts.
    depth: usize,
    level: usize,
    line: Option<TextSize>,
    // Where the item in each open bracket starts, once it has a token.
    items: Vec<Option<TextSize>>,
    // F-strings open, and where the outermost one starts.
    fstrings: usize,
    fstring_start: TextSize,
}

impl Walk<'_> {
    fn place(&mut self, token: &Token) {
        let range = match token.kind() {
            TokenKind::Indent => {
                self.level += 1;
                return;
            }
            TokenKind::Dedent => {
                self.level = self.level.saturating_sub(1);
                return;
            }
            TokenKind::Newline => {
                self.end_line(token.start());
                return;
            }
            TokenKind::Comment => {
                self.comment(token.range());
                return;
            }
            TokenKind::FStringStart | TokenKind::TStringStart => {
                if self.fstrings == 0 {
                    self.fstring_start = token.start();
                }
                self.fstrings += 1;
                return;
            }
            kind if kind.is_interpolated_string_end() => {
                self.fstrings -= 1;
                if self.fstrings > 0 {
                    return;
                }
                TextRange::new(self.fstring_start, token.end())
            }
            kind if self.fstrings > 0 || kind.is_trivia() => return,
            _ => token.range(),
        };
        let kind = token.kind();
        let start = *self.line.get_or_insert(range.start());
        if kind != TokenKind::Comma
            && !closes_bracket(kind)
            && let Some(item) = self.items.last_mut()
        {
            item.get_or_insert(range.start());
        }
        let item = self.items.last().copied().flatten();
        self.placed.push(Placed { kind, range, depth: self.depth, level: self.level, line: start, item });
        if opens_bracket(kind) {
            self.depth += 1;
            self.items.push(None);
        } else if closes_bracket(kind) {
            self.depth = self.depth.saturating_sub(1);
            self.items.pop();
        } else if kind == TokenKind::Comma
            && let Some(item) = self.items.last_mut()
        {
            *item = None;
        }
    }

    // The formatter splits a logical line wider than its line width, and a pragma on one then trails whatever part
    // ends last instead of the code it was written for.
    fn end_line(&mut self, end: TextSize) {
        if let Some(start) = self.line.take()
            && self.pragmas > 0
        {
            let line_tokens = &self.placed[self.placed.partition_point(|token| token.line < start)..];
            let code = without_comments(self.source, TextRange::new(start, end), &self.comments);
            if self.widths.indent * self.level + logical_width(&code, line_tokens) > self.widths.line
                && !keeps_its_line(self.source, line_tokens, self.widths, self.level)
            {
                self.moved_pragmas += self.pragmas;
            }
        }
        (self.pragmas, self.comments) = (0, Vec::new());
    }

    fn comment(&mut self, comment: TextRange) {
        self.frozen.note(self.source, comment, self.line);
        let Some(start) = self.line.filter(|_| !self.frozen.contains(comment.start())) else {
            return;
        };
        self.comments.push(comment);
        self.pragmas += usize::from(is_pragma(&self.source[comment]));
        let line_tokens = &self.placed[self.placed.partition_point(|token| token.line < start)..];
        let overlong = overlong_comment(self.source, line_tokens, comment, self.level, self.depth, self.widths);
        self.overlong_comments += usize::from(overlong);
    }
}

fn opens_bracket(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace)
}

fn closes_bracket(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
}

// A name, a string or an f-string: a token the formatter never splits.
fn is_atom(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::String) || kind.is_interpolated_string_end()
}

// The formatter leaves a lone value that does not fit even in parentheses of its own after a target it cannot split,
// as in `name = "..."`, `name: str = "..."` or `return "..."`, on the line it is on; after `a = b = `, `items[0] = `
// or `name: Final[str] = ` it wraps the value in parentheses anyway. `line` holds a token at least.
fn keeps_its_line(source: &str, line: &[Placed], widths: Widths, level: usize) -> bool {
    // A value in parentheses the formatter drops counts as the value alone.
    let bare = line.len() >= 3 && redundant_parentheses(line, line.len() - 2);
    let (value, head) = if bare {
        (&line[line.len() - 2], &line[..line.len() - 3])
    } else {
        (&line[line.len() - 1], &line[..line.len() - 1])
    };
    let wrapped = widths.indent * (level + 1) + source[value.range].chars().count();
    (is_atom(value.kind) || value.kind == TokenKind::Int)
        && wrapped > widths.line
        && head.last().is_some_and(|token| matches!(token.kind, TokenKind::Equal | TokenKind::Return))
        && head.iter().all(|token| !opens_bracket(token.kind))
        && head.iter().filter(|token| token.kind == TokenKind::Equal).count() <= 1
}

// The width of a logical line's code on one line. The formatter drops the parentheses of an import that fits without
// them and has no magic trailing comma.
fn logical_width(code: &str, line: &[Placed]) -> usize {
    let kinds: Vec<TokenKind> = line.iter().map(|token| token.kind).collect();
    let parenthesized_import = kinds.first() == Some(&TokenKind::From)
        && kinds.windows(2).any(|pair| pair == [TokenKind::Import, TokenKind::Lpar])
        && kinds.ends_with(&[TokenKind::Rpar])
        && !kinds.ends_with(&[TokenKind::Comma, TokenKind::Rpar]);
    collapsed(code) - 2 * usize::from(parenthesized_import)
}

// The code in `range` with each comment in it replaced by a space.
fn without_comments(source: &str, range: TextRange, comments: &[TextRange]) -> String {
    let mut code = String::new();
    let mut from = range.start();
    for comment in comments {
        code.push_str(&source[TextRange::new(from, comment.start())]);
        code.push(' ');
        from = comment.end();
    }
    code.push_str(&source[TextRange::new(from, range.end())]);
    code
}

// Whether the atom at `index` ends up on a line longer than the limit whatever the layout, one Ruff reports: its line
// has whitespace, as Ruff's E501 skips a line that is a single word, and does not end in a URL starting within the
// limit. A string over several lines is judged by its first line, which stays after the code before it.
fn overlong(source: &str, placed: &[Placed], index: usize, widths: Widths) -> bool {
    let atom = &placed[index];
    if !is_atom(atom.kind) {
        return false;
    }
    let kind_at = |offset: isize| {
        index.checked_add_signed(offset).and_then(|at| placed.get(at)).map_or(TokenKind::Newline, |token| token.kind)
    };
    let text = &source[atom.range];
    let first_line = text.lines().next().unwrap_or_default();
    let multiline = first_line.len() < text.len();
    let glue = if multiline { 0 } else { glued(kind_at) };
    let width = first_line.chars().count() + glue;
    let base = widths.indent * atom.level;
    let spaced = first_line.contains(char::is_whitespace);
    let depth = atom.depth - usize::from(redundant_parentheses(placed, index) && atom.depth > 0);
    let pinned = atom.range.start() == atom.line
        || matches!(kind_at(-1), TokenKind::Def | TokenKind::Class | TokenKind::Import | TokenKind::From);
    // A split operation puts its operator at the start of the operand's line.
    let operator = if starts_split_operand(kind_at(-1), kind_at(-2)) {
        source[placed[index - 1].range].chars().count() + 1
    } else {
        0
    };
    let (before, reported) = if pinned || depth == 0 {
        if !pinned && !multiline && base + widths.indent + operator + width <= widths.line {
            return false;
        }
        let (start, wrapped) = continuation_start(placed, index, depth, atom.line);
        let prefix = &source[TextRange::new(start, atom.range.start())];
        let gap = usize::from(prefix.ends_with(char::is_whitespace) && !prefix.trim().is_empty());
        let indent = widths.indent * usize::from(wrapped);
        (base + indent + collapsed(prefix) + gap, spaced || !prefix.trim().is_empty())
    } else {
        let key = atom
            .item
            .filter(|_| matches!(kind_at(-1), TokenKind::Colon | TokenKind::Equal))
            .map(|item| &source[TextRange::new(item, atom.range.start())]);
        let key_width = key.map_or(0, |key| collapsed(key) + usize::from(key.trim_end().ends_with(':')));
        (base + widths.indent * depth + key_width + operator, spaced || key.is_some() || operator > 0)
    };
    let total = before + width;
    reported && total > widths.limit && !ends_in_url(first_line, total - glue, widths.limit)
}

// Whether `text`, ending a line `total` wide, ends in a URL that starts within the limit, a line Ruff's E501 skips.
fn ends_in_url(text: &str, total: usize, limit: usize) -> bool {
    let last_chunk = text.rsplit(char::is_whitespace).next().unwrap_or_default();
    last_chunk.contains("://") && total - last_chunk.chars().count() <= limit
}

// Where the line holding the token at `index` starts once the formatter splits what comes before it on its logical
// line, from `floor` on: at the close of the last bracket, as in `) from error`, an empty one aside, or at the last
// operator of an operation at its depth, as in `+ "text"`. Whether that operation goes in parentheses of its own, as
// one outside brackets does, comes with it.
fn continuation_start(placed: &[Placed], index: usize, depth: usize, floor: TextSize) -> (TextSize, bool) {
    let line = placed[index].line;
    (1..index)
        .rev()
        .take_while(|&at| placed[at].line == line && placed[at].range.start() >= floor)
        .find_map(|at| {
            let (token, before) = (&placed[at], &placed[at - 1]);
            if closes_bracket(token.kind) && token.depth == depth + 1 && !opens_bracket(before.kind) {
                Some((token.range.start(), false))
            } else if token.depth == depth && starts_split_operand(token.kind, before.kind) {
                Some((token.range.start(), depth == 0))
            } else {
                None
            }
        })
        .unwrap_or((floor, false))
}

// An operator the formatter starts an operand's line with when it splits the operation: a binary operator save `**`,
// which it keeps tight, a boolean or comparison operator, or the `if` and `else` of a conditional expression. An
// augmented assignment is a statement, which it does not split there.
fn starts_split_operand(kind: TokenKind, before: TokenKind) -> bool {
    let operand_ends = is_atom(before)
        || closes_bracket(before)
        || before.is_singleton()
        || matches!(before, TokenKind::Int | TokenKind::Float | TokenKind::Complex);
    let binary = kind.as_binary_operator().is_some_and(|operator| operator != Operator::Pow)
        || kind.as_bool_operator().is_some()
        || matches!(
            kind,
            TokenKind::If
                | TokenKind::Else
                | TokenKind::In
                | TokenKind::Is
                | TokenKind::Not
                | TokenKind::EqEqual
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
        );
    operand_ends && binary
}

// Whether a comment ends up on a line longer than the limit whatever the layout. One on its own line inside brackets
// stays there, at the depth of the items around it; one after code stays after the code before it: the bracket it
// follows with what comes before that, the item it ends, or the closing bracket. Ruff's E501 leaves out a pragma.
fn overlong_comment(
    source: &str,
    line: &[Placed],
    comment: TextRange,
    level: usize,
    depth: usize,
    widths: Widths,
) -> bool {
    let text = &source[comment];
    let width = if has_leading_content(comment.start(), source) {
        let Some(mut last) = line.len().checked_sub(1).filter(|_| !is_pragma(text)) else {
            return false;
        };
        // The formatter drops parentheses around a lone value, which leaves the comment after the value.
        let unwrapped = line[last].kind == TokenKind::Rpar && last > 0 && redundant_parentheses(line, last - 1);
        last -= usize::from(unwrapped);
        let previous = &line[last];
        let previous_depth = previous.depth - usize::from(unwrapped);
        let (from, depth) = match previous.kind {
            kind if closes_bracket(kind) => (previous.range.start(), previous_depth - 1),
            TokenKind::String if last > 0 && line[last - 1].kind == TokenKind::String => {
                (previous.range.start(), previous_depth)
            }
            _ => {
                let item = if unwrapped { line[last - 1].item } else { previous.item };
                let (start, wrapped) = continuation_start(line, last, previous_depth, item.unwrap_or(previous.line));
                (start, previous_depth + usize::from(wrapped))
            }
        };
        let code = collapsed(&source[TextRange::new(from, previous.range.end())]);
        widths.indent * (level + depth) + code - usize::from(unwrapped && from < previous.range.start()) + 2
    } else if depth > 0 && text.contains(char::is_whitespace) {
        widths.indent * (level + depth)
    } else {
        return false;
    };
    let total = width + text.chars().count();
    total > widths.limit && !ends_in_url(text, total, widths.limit)
}

// Whether the token at `index` sits alone in parentheses that are not a call's, ones the formatter removes when they do
// not help.
fn redundant_parentheses(placed: &[Placed], index: usize) -> bool {
    let kind_at = |at: Option<usize>| at.and_then(|at| placed.get(at)).map_or(TokenKind::Newline, |token| token.kind);
    kind_at(index.checked_sub(1)) == TokenKind::Lpar
        && kind_at(Some(index + 1)) == TokenKind::Rpar
        && !matches!(
            kind_at(index.checked_sub(2)),
            TokenKind::Identifier | TokenKind::Rpar | TokenKind::Rsqb | TokenKind::String
        )
}

// The width of what stays on an atom's line after it: a comma, or the bracket it opens, with the rest of the signature
// when that bracket is empty: `def name() -> (`, as the formatter splits the return annotation instead.
fn glued(kind_at: impl Fn(isize) -> TokenKind) -> usize {
    match (kind_at(1), kind_at(2)) {
        (TokenKind::Lpar, TokenKind::Rpar) if kind_at(-1) == TokenKind::Def => match kind_at(3) {
            TokenKind::Rarrow => "() -> (".len(),
            _ => "():".len(),
        },
        (TokenKind::Comma | TokenKind::Lpar | TokenKind::Lsqb, _) => 1,
        _ => 0,
    }
}

// The width of code with every run of whitespace, line breaks included, reduced to one space, and none left just
// inside a bracket, as the formatter writes it.
fn collapsed(code: &str) -> usize {
    let mut width = 0;
    let mut previous: Option<&str> = None;
    for word in code.split_whitespace() {
        if previous.is_some_and(|previous| !previous.ends_with(['(', '[', '{']) && !word.starts_with([')', ']', '}'])) {
            width += 1;
        }
        width += word.chars().count();
        previous = Some(word);
    }
    width
}

// The code `ruff format` leaves alone: a file its settings exclude, from an own-line `# fmt: off` to the next
// `# fmt: on`, and the statement a `# fmt: skip` ends.
#[derive(Default)]
struct Frozen {
    whole: bool,
    ranges: Vec<TextRange>,
    off: Option<TextSize>,
}

impl Frozen {
    fn note(&mut self, source: &str, comment: TextRange, line: Option<TextSize>) {
        let own_line = !has_leading_content(comment.start(), source);
        match FormatSuppression::from_comment(&source[comment]) {
            Some(FormatSuppression::Off) if own_line => {
                self.off.get_or_insert(comment.start());
            }
            Some(FormatSuppression::On) if own_line => {
                if let Some(off) = self.off.take() {
                    self.ranges.push(TextRange::new(off, comment.end()));
                }
            }
            Some(FormatSuppression::Skip) => {
                if let Some(start) = line {
                    self.ranges.push(TextRange::new(start, comment.end()));
                }
            }
            _ => {}
        }
    }

    fn finish(mut self, source: &str) -> Self {
        if self.whole {
            self.ranges = vec![TextRange::up_to(TextSize::of(source))];
        } else if let Some(off) = self.off.take() {
            self.ranges.push(TextRange::new(off, TextSize::of(source)));
        }
        self
    }

    fn contains(&self, offset: TextSize) -> bool {
        self.whole
            || self.off.is_some_and(|off| off <= offset)
            || self.ranges.iter().any(|range| range.contains_inclusive(offset))
    }

    // Lines over the limit as they stand, save one that is a single word once its trailing pragma comment is left
    // out, as Ruff's E501 skips those.
    fn overlong_lines(&self, source: &str, limit: usize) -> usize {
        self.ranges
            .iter()
            .flat_map(|range| source[source.full_lines_range(*range)].lines())
            .filter(|line| {
                let code = line.rfind('#').filter(|&at| is_pragma(&line[at..])).map_or(*line, |at| &line[..at]);
                let code = code.trim_end();
                code.chars().count() > limit && code.trim_start().contains(char::is_whitespace)
            })
            .count()
    }
}

fn is_pragma(comment: &str) -> bool {
    is_pragma_comment(comment) || comment.trim_start_matches('#').trim_start().starts_with("simplint:")
}
