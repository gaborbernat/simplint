use ruff_text_size::TextSize;

use super::blocks::{Classified, classify_docstring};
use super::lines::{Line, TAB_WIDTH, columns, indent_columns, lines};
use super::roles::{myst_references, references, simplename_len};

const BLANKS: [char; 2] = [' ', '\t'];

// markdown-it lays out a line's indentation on tab stops four columns apart.
const MYST_TAB: usize = 4;

// A directive of a page with the argument and options under it: `.. name:: argument` with `:key: value` lines in
// reStructuredText, or `{name} argument` behind a fence in MyST with `:key: value` lines or a `---` block under it. Each
// line of the argument is a signature, as Sphinx reads them, so `.. data:: IDLE` followed by `SEND_RESPONSE` has two.
pub(crate) struct Directive {
    pub(crate) name: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) options: Vec<(String, String)>,
}

// The directives of a page, and the targets of the object roles in its prose.
#[derive(Default)]
pub(crate) struct Markup {
    pub(crate) directives: Vec<Directive>,
    pub(crate) references: Vec<String>,
}

// The directives of a walk and the paragraphs of its prose, which a role is read from: a paragraph holds no code,
// comment, option or raw content.
#[derive(Default)]
pub(crate) struct Walked<'a> {
    markup: Markup,
    pub(crate) paragraphs: Vec<Vec<Line<'a>>>,
}

impl Walked<'_> {
    fn finish(self, roles: fn(&[Line]) -> Vec<String>) -> Markup {
        let mut markup = self.markup;
        for paragraph in &self.paragraphs {
            markup.references.extend(roles(paragraph));
        }
        markup
    }
}

// The lines of a page with the code the docstring classifier finds in them. A page has no opening quotes, so the
// classifier reads its first line as the others: a blank line in front of the text leaves each line its own indent.
pub(crate) fn page_lines(text: &str) -> Vec<Classified<'_>> {
    let docstring = format!("\n{text}");
    let classified = classify_docstring(&docstring);
    lines(text)
        .into_iter()
        .zip(classified.iter().skip(1))
        .map(|(line, found)| Classified { line, code: found.code, opens: found.opens, verbatim: found.verbatim })
        .collect()
}

pub(crate) fn rst_markup(lines: &[Classified]) -> Markup {
    rst_walk(lines).finish(references)
}

// The spec of a directive is not known to the scanner, so the lines under any directive are its argument, and a
// directive without arguments reads them as content, as docutils does. Either way they are prose a role can sit in.
pub(crate) fn rst_walk<'a>(lines: &[Classified<'a>]) -> Walked<'a> {
    let mut walked = Walked::default();
    let mut walk = RstWalk::default();
    for classified in lines {
        walk.line(classified, &mut walked);
    }
    walk.finish(&mut walked);
    walked
}

pub(crate) fn myst_markup(text: &str) -> Markup {
    let mut walked = Walked::default();
    myst_block(&myst_page(text), &mut walked);
    walked.finish(myst_references)
}

#[derive(Default)]
struct RstWalk<'a> {
    paragraph: Vec<Line<'a>>,
    // The column of a comment, raw or math directive, whose more indented lines are not prose.
    skip: Option<usize>,
    argument: Option<Argument>,
    // The column and the text after the first `|` of a substitution definition whose directive is on a later line.
    substitution: Option<(usize, String)>,
    grid: Option<Grid<'a>>,
    // Whether this walks the text of a grid cell, which holds no table of its own.
    cell: bool,
    simple_table: Option<SimpleTable>,
}

// The cells of a grid table's open rows, each walked on its own so a directive and its options can sit in one cell.
// A cell is keyed by the column its text starts at, so a cell spanning rows keeps its walker across them.
#[derive(Default)]
struct Grid<'a> {
    cells: Vec<(usize, RstWalk<'a>)>,
}

// A simple table: where each column starts, in characters, as the runs of `=` on its top border give them, and how
// many borders the table has passed. The second border ends it, as does a blank line.
struct SimpleTable {
    columns: Vec<usize>,
    borders: u8,
}

// The lines under a directive, indented more than its line: the argument, then the options from the first `:key:`
// line on. The options are at the indentation of the block's first line after the directive, and a line indented
// further than an option belongs to its value.
struct Argument {
    directive: usize,
    indent: usize,
    block_indent: Option<usize>,
    option_column: usize,
}

enum Explicit<'a> {
    Comment,
    Directive { name: &'a str, rest: &'a str },
    Substitution(&'a str),
    Target,
    Other,
}

impl<'a> RstWalk<'a> {
    fn cell() -> Self {
        Self { cell: true, ..Self::default() }
    }

    fn line(&mut self, classified: &Classified<'a>, walked: &mut Walked<'a>) {
        let line = classified.line;
        if line.text.trim().is_empty() {
            self.flush(walked);
            self.end_grid(walked);
            self.argument = None;
            self.substitution = None;
            self.simple_table = None;
            return;
        }
        if self.argument.as_ref().is_some_and(|argument| line.indent <= argument.indent) {
            self.argument = None;
        }
        if self.skip.is_some_and(|indent| line.indent <= indent) {
            self.skip = None;
        }
        if self.substitution.as_ref().is_some_and(|(indent, _)| line.indent <= *indent) {
            self.substitution = None;
        }
        // The docstring classifier reads explicit markup and its continuation as not prose, while this walk reads
        // directives and substitution definitions from those lines itself.
        let code =
            classified.code && self.substitution.is_none() && !(classified.opens && explicit(line.text).is_some());
        if let Some(argument) = self.argument.as_mut() {
            let argument_prose = argument_line(&mut walked.markup, argument, line.text.trim(), line.indent, false);
            if argument_prose && !code && self.skip.is_none() {
                self.paragraph.push(line);
            } else {
                self.flush(walked);
            }
            return;
        }
        if code || self.skip.is_some() {
            self.flush(walked);
            return;
        }
        if let Some((indent, mut text)) = self.substitution.take() {
            text.push(' ');
            text.push_str(line.text.trim());
            if let Some((name, rest)) = substitution_directive(&text) {
                self.start_directive(name, rest, indent, walked);
            } else {
                self.substitution = Some((indent, text));
            }
            self.flush(walked);
            return;
        }
        let trimmed = line.text.trim();
        if let Some(grid) = self.grid.as_mut() {
            if trimmed.starts_with('+') {
                grid.flush(walked);
                return;
            }
            if trimmed.starts_with('|') {
                grid.row(line, classified, walked);
                return;
            }
            self.end_grid(walked);
        }
        if !self.cell && is_grid_border(trimmed) {
            self.flush(walked);
            self.grid = Some(Grid::default());
            return;
        }
        if self.simple_table_line(line, walked) {
            return;
        }
        if is_simple_top(trimmed) {
            self.flush(walked);
            self.simple_table = Some(SimpleTable { columns: simple_columns(line.text), borders: 0 });
            return;
        }
        match explicit(line.text) {
            Some((Explicit::Comment | Explicit::Target, column)) => {
                self.flush(walked);
                self.skip = Some(column);
            }
            Some((Explicit::Directive { name, rest }, column)) => {
                self.flush(walked);
                if !is_raw_or_math(name) {
                    self.paragraph.push(line);
                }
                self.start_directive(name, rest, column, walked);
            }
            Some((Explicit::Substitution(text), column)) => {
                self.flush(walked);
                self.substitution = Some((column, text.to_owned()));
            }
            Some((Explicit::Other, _)) | None => {
                if starts_block(line.text) || self.starts_definition(line) {
                    self.flush(walked);
                }
                self.paragraph.push(line);
            }
        }
    }

    // A line of an open simple table: a border counts toward its end, and a row's cell that holds a directive starts one.
    fn simple_table_line(&mut self, line: Line<'a>, walked: &mut Walked<'a>) -> bool {
        let Some(table) = self.simple_table.as_mut() else {
            return false;
        };
        let trimmed = line.text.trim();
        if is_simple_border(trimmed) {
            table.borders += 1;
            if table.borders == 2 {
                self.simple_table = None;
            }
            self.flush(walked);
            return true;
        }
        let offsets: Vec<usize> = line.text.char_indices().map(|(at, _)| at).chain([line.text.len()]).collect();
        let cells: Vec<(&'a str, &'a str)> = table
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, &start)| {
                let end = table.columns.get(index + 1).copied();
                match explicit(cell_text(line.text, &offsets, start, end))? {
                    (Explicit::Directive { name, rest }, _) => Some((name, rest)),
                    _ => None,
                }
            })
            .collect();
        for (name, rest) in cells {
            self.start_directive(name, rest, line.indent, walked);
        }
        self.paragraph.push(line);
        true
    }

    // An indented line after a term, a paragraph line that is not a field or a list item, is the term's definition.
    fn starts_definition(&self, line: Line<'a>) -> bool {
        self.paragraph.first().is_some_and(|first| line.indent > first.indent && !starts_block(first.text))
    }

    fn start_directive(&mut self, name: &str, rest: &str, column: usize, walked: &mut Walked<'a>) {
        let directive = walked.markup.directives.len();
        walked.markup.directives.push(Directive { name: name.to_owned(), arguments: Vec::new(), options: Vec::new() });
        let mut argument = Argument { directive, indent: column, block_indent: None, option_column: 0 };
        let rest = rest.trim();
        if !rest.is_empty() {
            argument_line(&mut walked.markup, &mut argument, rest, column, true);
        }
        if is_raw_or_math(name) {
            self.skip = Some(column);
        }
        self.argument = Some(argument);
    }

    fn flush(&mut self, walked: &mut Walked<'a>) {
        flush(&mut self.paragraph, walked);
    }

    fn end_grid(&mut self, walked: &mut Walked<'a>) {
        if let Some(grid) = self.grid.take() {
            for (_, mut cell) in grid.cells {
                cell.flush(walked);
            }
        }
    }

    fn finish(&mut self, walked: &mut Walked<'a>) {
        self.end_grid(walked);
        self.flush(walked);
    }
}

impl<'a> Grid<'a> {
    fn flush(&mut self, walked: &mut Walked<'a>) {
        for (_, cell) in &mut self.cells {
            cell.flush(walked);
        }
    }

    // A row's cells are the text between its bars, each walked by the cell of the column it starts at.
    fn row(&mut self, line: Line<'a>, classified: &Classified, walked: &mut Walked<'a>) {
        let bars: Vec<usize> = line.text.match_indices('|').map(|(at, _)| at).collect();
        for bounds in bars.windows(2) {
            let (start, end) = (bounds[0] + 1, bounds[1]);
            let key = line.text[..start].chars().count();
            let text = &line.text[start..end];
            let cell_line = Line {
                start: line.start + TextSize::of(&line.text[..start]),
                raw: text,
                text,
                indent: indent_columns(text, TAB_WIDTH),
            };
            let index = self.cells.iter().position(|(column, _)| *column == key).unwrap_or_else(|| {
                self.cells.push((key, RstWalk::cell()));
                self.cells.len() - 1
            });
            let cell = Classified {
                line: cell_line,
                code: classified.code,
                opens: classified.opens,
                verbatim: classified.verbatim,
            };
            self.cells[index].1.line(&cell, walked);
        }
    }
}

// The text of a line from one character column to the next, or to the end of the line, given the byte offset of each
// character and the end of the line.
fn cell_text<'t>(text: &'t str, offsets: &[usize], start: usize, end: Option<usize>) -> &'t str {
    let from = offsets.get(start).copied().unwrap_or(text.len());
    let to = end.and_then(|end| offsets.get(end).copied()).unwrap_or(text.len());
    &text[from..to]
}

fn flush<'a>(paragraph: &mut Vec<Line<'a>>, walked: &mut Walked<'a>) {
    if !paragraph.is_empty() {
        walked.paragraphs.push(std::mem::take(paragraph));
    }
}

// Records a line under a directive, and tells whether it is prose: its argument is, its options are not. The first
// line after the directive gives the block's indentation, and the text on the directive's line is at column zero.
fn argument_line(markup: &mut Markup, argument: &mut Argument, text: &str, indent: usize, header: bool) -> bool {
    let column = if header { 0 } else { indent.saturating_sub(*argument.block_indent.get_or_insert(indent)) };
    let directive = &mut markup.directives[argument.directive];
    if let Some((_, value)) = directive.options.last_mut()
        && column > argument.option_column
    {
        push_joined(value, text);
        return false;
    }
    if let Some((name, value)) = field_marker(text) {
        directive.options.push((unescape_name(name), value.to_owned()));
        argument.option_column = column;
        return false;
    }
    if !directive.options.is_empty() {
        return false;
    }
    push_signature(&mut directive.arguments, text);
    true
}

// A line of an argument is a signature of its own, unless the line before ends in a backslash, which joins them.
fn push_signature(arguments: &mut Vec<String>, text: &str) {
    match arguments.last_mut() {
        Some(last) if last.ends_with('\\') => {
            last.pop();
            push_joined(last, text);
        }
        _ => arguments.push(text.to_owned()),
    }
}

fn push_joined(target: &mut String, text: &str) {
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(text);
}

fn is_raw_or_math(name: &str) -> bool {
    name.eq_ignore_ascii_case("raw") || name.eq_ignore_ascii_case("math")
}

// The top border of a grid table, `+-...-+` with dashes and pluses between, as docutils' `grid_table_top_pat` reads it.
fn is_grid_border(trimmed: &str) -> bool {
    trimmed.len() >= 5
        && trimmed.starts_with("+-")
        && trimmed.ends_with("-+")
        && trimmed.chars().all(|char| char == '+' || char == '-')
}

// The top border of a simple table: runs of `=` separated by spaces, two or more of them, as docutils'
// `simple_table_top_pat` reads it.
fn is_simple_top(trimmed: &str) -> bool {
    let runs: Vec<&str> = trimmed.split(' ').filter(|run| !run.is_empty()).collect();
    runs.len() >= 2 && runs.iter().all(|run| run.bytes().all(|byte| byte == b'='))
}

// A border inside or closing a simple table: `=` with spaces, as docutils' `simple_table_border_pat` reads it.
fn is_simple_border(trimmed: &str) -> bool {
    trimmed.starts_with('=') && trimmed.chars().all(|char| char == '=' || char == ' ')
}

// Where each run of `=` on a simple table's top border starts, in characters: the columns the rows are cut into.
fn simple_columns(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut previous = ' ';
    for (at, char) in text.chars().enumerate() {
        if char == '=' && previous != '=' {
            starts.push(at);
        }
        previous = char;
    }
    starts
}

// Docutils reads a field name with the backslashes that escape its characters removed.
fn unescape_name(name: &str) -> String {
    let mut unescaped = String::with_capacity(name.len());
    let mut chars = name.chars();
    while let Some(char) = chars.next() {
        if char == '\\' {
            unescaped.extend(chars.next());
        } else {
            unescaped.push(char);
        }
    }
    unescaped
}

// What a line opens when it starts with `..`, as docutils' explicit markup recognition sorts it, with the column the
// `..` is at: a footnote, a target and a substitution definition are not comments, and a directive is one only with
// `::` after its name.
fn explicit(text: &str) -> Option<(Explicit<'_>, usize)> {
    let body = strip_list_marker(text.trim_start());
    let column = columns(&text[..text.len() - body.len()], TAB_WIDTH);
    let rest = body.strip_prefix("..")?;
    if !(rest.is_empty() || rest.starts_with(BLANKS)) {
        return None;
    }
    let rest = rest.trim_start_matches(BLANKS);
    let found = if rest.starts_with('[') {
        Explicit::Other
    } else if rest.strip_prefix('_').is_some_and(|target| !target.is_empty() && !target.starts_with(BLANKS)) {
        Explicit::Target
    } else if let Some(substitution) =
        rest.strip_prefix('|').filter(|text| !text.is_empty() && !text.starts_with(BLANKS))
    {
        match substitution_directive(substitution) {
            Some((name, rest)) => Explicit::Directive { name, rest },
            None => Explicit::Substitution(substitution),
        }
    } else {
        match directive_header(rest) {
            Some((name, rest)) => Explicit::Directive { name, rest },
            None => Explicit::Comment,
        }
    };
    Some((found, column))
}

// The name and the directive of a substitution definition, whose text follows its first `|`: the name runs to a `|`
// that follows a non-space, and the directive follows that `|` after a space.
fn substitution_directive(text: &str) -> Option<(&str, &str)> {
    let close =
        text.char_indices().skip(1).find(|&(at, char)| char == '|' && !text[..at].ends_with(char::is_whitespace))?.0;
    let rest = text[close + 1..].strip_prefix(BLANKS)?;
    directive_header(rest.trim_start_matches(BLANKS))
}

// The name and the text after the `::` of a directive, which needs no space before the `::` and a space or the end of
// the line after it.
fn directive_header(text: &str) -> Option<(&str, &str)> {
    let name_length = simplename_len(text);
    let after_name = &text[name_length..];
    let rest = after_name.strip_prefix(BLANKS).unwrap_or(after_name).strip_prefix("::")?;
    (name_length > 0 && (rest.is_empty() || rest.starts_with(BLANKS)))
        .then(|| (&text[..name_length], rest.trim_start_matches(BLANKS)))
}

// The text of a list item after its bullet or enumerator, which may open a directive.
fn strip_list_marker(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix(['-', '+', '*', '•', '‣', '⁃']) {
        return if rest.is_empty() || rest.starts_with(BLANKS) { rest.trim_start_matches(BLANKS) } else { text };
    }
    match enumerator_len(text) {
        Some(length) => text[length..].trim_start_matches(BLANKS),
        None => text,
    }
}

// Whether a line starts a list item or a field, which begins a paragraph of its own, as docutils reads them.
fn starts_block(text: &str) -> bool {
    let body = text.trim_start();
    strip_list_marker(body).len() < body.len() || field_marker(body).is_some()
}

// The length of an enumerator docutils reads as a list marker: `#`, digits, one letter or roman numerals, written
// `(x)`, `x)` or `x.` and followed by a space or the end of the line.
fn enumerator_len(text: &str) -> Option<usize> {
    let (parens, body) = match text.strip_prefix('(') {
        Some(body) => (true, body),
        None => (false, text),
    };
    let end = body.find([')', '.'])?;
    let suffix = body[end..].chars().next()?;
    let closed = if parens { suffix == ')' } else { suffix == ')' || suffix == '.' };
    let length = usize::from(parens) + end + 1;
    (closed && is_enumerator(&body[..end]) && (text[length..].is_empty() || text[length..].starts_with(BLANKS)))
        .then_some(length)
}

fn is_enumerator(token: &str) -> bool {
    let roman = |numerals: &str| !token.is_empty() && token.chars().all(|char| numerals.contains(char));
    token == "#"
        || (!token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit()))
        || (token.len() == 1 && token.chars().all(|char| char.is_ascii_alphabetic()))
        || roman("ivxlcdm")
        || roman("IVXLCDM")
}

// A field marker `:key: value` of reStructuredText, whose key neither starts with a colon or a blank nor ends with one.
fn field_marker(text: &str) -> Option<(&str, &str)> {
    let body = text.strip_prefix(':')?;
    if body.starts_with([':', ' ', '\t']) {
        return None;
    }
    let (name, value) = option_pair(body)?;
    (!name.ends_with(BLANKS)).then(|| (name, value.trim()))
}

// The key of a `key: value` line ends at the first colon followed by a blank or the end of the line, as MyST reads
// its options.
fn option_pair(text: &str) -> Option<(&str, &str)> {
    let (at, _) =
        text.match_indices(':').find(|&(at, _)| text[at + 1..].is_empty() || text[at + 1..].starts_with(BLANKS))?;
    Some((&text[..at], &text[at + 1..]))
}

#[derive(Clone, Copy)]
struct Fence<'a> {
    marker: char,
    length: usize,
    info: &'a str,
}

impl<'a> Fence<'a> {
    // A fence of at least three backticks, tildes or colons, opened by at most three spaces; a backtick fence's info
    // holds no backtick, as that would be inline code.
    fn opening(text: &'a str, base: usize) -> Option<Self> {
        let body = indented(text, base)?;
        let marker = body.chars().next().filter(|char| matches!(char, '`' | '~' | ':'))?;
        let length = body.chars().take_while(|&char| char == marker).count();
        let info = body[length..].trim();
        (length >= 3 && !(marker == '`' && info.contains('`'))).then_some(Self { marker, length, info })
    }

    // The number of lines of the content before the closing fence, or of all of them when no fence closes it.
    fn content_length(self, content: &[Line], base: usize) -> usize {
        content.iter().position(|line| self.closes(line.text, base)).unwrap_or(content.len())
    }

    fn closes(self, text: &str, base: usize) -> bool {
        let Some(body) = indented(text, base) else {
            return false;
        };
        let run = body.chars().take_while(|&char| char == self.marker).count();
        run >= self.length && body[run..].trim().is_empty()
    }
}

// The indentation of a MyST line in columns, and the text after it.
fn myst_indent(text: &str) -> (usize, &str) {
    let body = text.trim_start_matches(BLANKS);
    (columns(&text[..text.len() - body.len()], MYST_TAB), body)
}

// The text after the indentation of at most three columns past `base`, the content indentation of the list item the
// line belongs to.
fn indented(text: &str, base: usize) -> Option<&str> {
    let (indent, body) = myst_indent(text);
    (indent.saturating_sub(base) <= 3).then_some(body)
}

// A thematic break: three or more of one of `*`, `-` or `_`, with spaces between them. markdown-it reads it before a
// list item, so `* * *` is not a bullet.
fn is_thematic_break(body: &str) -> bool {
    let Some(marker) = body.chars().next().filter(|char| matches!(char, '*' | '-' | '_')) else {
        return false;
    };
    body.chars().filter(|char| *char == marker).count() >= 3
        && body.chars().all(|char| char == marker || char == ' ' || char == '\t')
}

// An ATX heading: one to six `#` followed by a blank or the end of the line.
fn is_heading(body: &str) -> bool {
    let hashes = body.bytes().take_while(|&byte| byte == b'#').count();
    (1..=6).contains(&hashes) && (body.len() == hashes || body[hashes..].starts_with(BLANKS))
}

// Whether a line opens an HTML block: an opening or closing tag whose name ends at the end of the line or the tag.
fn html_start(body: &str) -> bool {
    let Some(rest) = body.strip_prefix('<') else {
        return false;
    };
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    let name = rest.chars().take_while(|char| char.is_ascii_alphanumeric() || *char == '-').count();
    name > 0 && rest[name..].chars().next().is_none_or(|char| matches!(char, ' ' | '\t' | '>' | '/'))
}

// A blockquote line of the page as written, whose `>` `unquote` has taken away from its text.
fn quote_line(raw: &str) -> bool {
    let (indent, body) = myst_indent(raw);
    indent <= 3 && body.starts_with('>')
}

// The lines that end a paragraph and so end a list item's lazy continuation: a list item, a fence, a heading, a quote,
// a thematic break and an HTML block, as CommonMark's paragraph interruptions have them.
fn interrupts_paragraph(text: &str) -> bool {
    let body = text.trim_start_matches(BLANKS);
    list_item(text, 0).is_some()
        || Fence::opening(text, 0).is_some()
        || is_heading(body)
        || body.starts_with('>')
        || is_thematic_break(body)
        || html_start(body)
}

// Whether a list item may start in the middle of a paragraph: it has content, and an ordered one starts at `1`.
fn interrupts_with_list(text: &str, rest: &str) -> bool {
    let body = text.trim_start();
    let number = body.bytes().take_while(u8::is_ascii_digit).count();
    !rest.trim().is_empty() && (number == 0 || &body[..number] == "1")
}

// The column where the content of a list item starts when the line opens one, with the text after its marker: a bullet
// or an ordered marker, then the spaces after it, of which one to four count, and the marker's one space when more
// follow or none does.
fn list_item(text: &str, base: usize) -> Option<(usize, &str)> {
    let (indent, body) = myst_indent(text);
    if indent.saturating_sub(base) > 3 || is_thematic_break(body) {
        return None;
    }
    let marker = match body.chars().next()? {
        '-' | '+' | '*' => 1,
        _ => {
            let digits = body.bytes().take_while(u8::is_ascii_digit).count();
            if !(1..=9).contains(&digits) || !matches!(body.as_bytes().get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
    };
    let after = &body[marker..];
    if !(after.is_empty() || after.starts_with(BLANKS)) {
        return None;
    }
    let blanks = after.len() - after.trim_start_matches(BLANKS).len();
    let width = columns(&after[..blanks], MYST_TAB);
    let (gap, rest) =
        if width == 0 || width > 4 { (1, after.get(1..).unwrap_or_default()) } else { (width, &after[blanks..]) };
    Some((indent + marker + gap, rest))
}

// The directives MyST reads the content of as code, so their lines hold no prose.
const CODE_DIRECTIVES: [&str; 11] = [
    "code-block",
    "code",
    "sourcecode",
    "code-cell",
    "doctest",
    "testcode",
    "testoutput",
    "ipython",
    "jupyter-execute",
    "eval-rst",
    "math",
];

// The lines of a MyST page with its front matter dropped and its blockquotes read as their content, so a quote's lines
// are read as the page's own, with the indentation of that content.
fn myst_page(text: &str) -> Vec<Line<'_>> {
    let mut page: Vec<Line> =
        lines(text).into_iter().map(|line| Line { indent: indent_columns(line.text, MYST_TAB), ..line }).collect();
    if page.first().is_some_and(|line| line.text.trim_end() == "---")
        && let Some(close) = page.iter().skip(1).position(|line| line.text.trim_end() == "---")
    {
        page.drain(..close + 2);
    }
    page.into_iter().map(unquote).collect()
}

fn unquote(line: Line<'_>) -> Line<'_> {
    let (indent, body) = myst_indent(line.text);
    match body.strip_prefix('>') {
        Some(rest) if indent <= 3 => {
            let text = rest.strip_prefix(' ').unwrap_or(rest);
            Line { text, indent: indent_columns(text, MYST_TAB), ..line }
        }
        _ => line,
    }
}

// The blocks of a MyST page, one per fence whose content is read as MyST: a block reads its lines in turn, and the
// content of a fence is a block of its own, which the walk reads once the block of the fence's line is paused.
struct MystBlock<'b, 'a> {
    lines: &'b [Line<'a>],
    index: usize,
    paragraph: Vec<Line<'a>>,
    after_blank: bool,
    // The content column of the list item the lines belong to.
    item: Option<usize>,
    // Whether the lines are an HTML block, which runs to the next blank line.
    html: bool,
}

impl<'b, 'a> MystBlock<'b, 'a> {
    fn new(lines: &'b [Line<'a>]) -> Self {
        Self { lines, index: 0, paragraph: Vec::new(), after_blank: true, item: None, html: false }
    }

    // A line is code when it is indented four columns past the content of the list item it belongs to, or past the
    // margin outside a list, and follows a blank line. Reads lines until a fence opens content to read as MyST, which
    // is returned for the walk to read next.
    fn step(&mut self, walked: &mut Walked<'a>) -> Option<&'b [Line<'a>]> {
        let lines = self.lines;
        while let Some(&line) = lines.get(self.index) {
            let index = self.index;
            self.index += 1;
            if line.text.trim().is_empty() {
                flush(&mut self.paragraph, walked);
                self.after_blank = true;
                self.html = false;
                continue;
            }
            if self.html {
                continue;
            }
            let continues = self.item.is_some_and(|content| line.indent >= content);
            // A line that continues the paragraph of a list item belongs to the item even when it is not indented to it.
            let lazy = !continues
                && !self.after_blank
                && self.item.is_some()
                && !interrupts_paragraph(line.text)
                && !quote_line(line.raw);
            let base = if continues { self.item.unwrap_or_default() } else { 0 };
            if !continues && !lazy {
                self.item = None;
            }
            let item = list_item(line.text, base)
                .filter(|(_, rest)| self.after_blank || interrupts_with_list(line.text, rest));
            if let Some((content, _)) = item {
                self.item = Some(content);
            }
            let opener = item
                .and_then(|(content, rest)| Fence::opening(rest, 0).map(|fence| (fence, content)))
                .or_else(|| Fence::opening(line.text, base).map(|fence| (fence, base)));
            if let Some((fence, fence_base)) = opener {
                flush(&mut self.paragraph, walked);
                let content_start = index + 1;
                let close = content_start + fence.content_length(&lines[content_start..], fence_base);
                self.index = close + 1;
                self.after_blank = true;
                if let Some(content) = myst_fence(fence, &lines[content_start..close], walked) {
                    return Some(content);
                }
                continue;
            }
            if self.after_blank && line.indent >= base + 4 {
                continue;
            }
            let body = line.text.trim_start_matches(BLANKS);
            if body.starts_with('%') {
                flush(&mut self.paragraph, walked);
                self.after_blank = false;
                continue;
            }
            if is_heading(body) {
                flush(&mut self.paragraph, walked);
                self.paragraph.push(line);
                flush(&mut self.paragraph, walked);
                self.after_blank = true;
                continue;
            }
            if html_start(body) {
                flush(&mut self.paragraph, walked);
                self.html = true;
                continue;
            }
            if is_thematic_break(body) {
                flush(&mut self.paragraph, walked);
                self.after_blank = true;
                continue;
            }
            self.paragraph.push(line);
            self.after_blank = false;
        }
        flush(&mut self.paragraph, walked);
        None
    }
}

// The MyST blocks are read with an explicit stack, so a page nested deeper than the thread's stack reads all the same.
fn myst_block<'a>(lines: &[Line<'a>], walked: &mut Walked<'a>) {
    let mut blocks = vec![MystBlock::new(lines)];
    while let Some(block) = blocks.last_mut() {
        match block.step(walked) {
            Some(content) => blocks.push(MystBlock::new(content)),
            None => _ = blocks.pop(),
        }
    }
}

// A fence that names a directive, `{name} argument`, records it with the options at the start of its content. The rest
// of the content is returned for the walk to read as MyST, unless the directive reads it as code; `eval-rst` holds
// reStructuredText, which is read here as such.
fn myst_fence<'b, 'a>(fence: Fence<'_>, content: &'b [Line<'a>], walked: &mut Walked<'a>) -> Option<&'b [Line<'a>]> {
    let (name, argument) = directive_info(fence.info)?;
    let argument = argument.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut directive = Directive {
        name: name.to_owned(),
        arguments: Vec::from_iter((!argument.is_empty()).then_some(argument)),
        options: Vec::new(),
    };
    let body = myst_options(content, &mut directive.options);
    walked.markup.directives.push(directive);
    if name == "eval-rst" {
        let text = body.iter().map(|line| line.text).collect::<Vec<_>>().join("\n");
        let nested = rst_markup(&page_lines(&text));
        walked.markup.directives.extend(nested.directives);
        walked.markup.references.extend(nested.references);
        None
    } else if CODE_DIRECTIVES.contains(&name) {
        None
    } else {
        Some(body)
    }
}

fn directive_info(info: &str) -> Option<(&str, &str)> {
    let inner = info.strip_prefix('{')?;
    let close = inner.find('}')?;
    let name = &inner[..close];
    (!name.is_empty() && !name.contains(char::is_whitespace)).then(|| (name, inner[close + 1..].trim()))
}

// The options of a directive's content: a `---` block up to the next line of dashes, or the `:key: value` lines it
// starts with, which end at a colon fence.
fn myst_options<'a, 'b>(content: &'b [Line<'a>], options: &mut Vec<(String, String)>) -> &'b [Line<'a>] {
    let Some(first) = content.first() else {
        return content;
    };
    if first.text.starts_with("---") {
        let close = content.iter().skip(1).position(|line| line.text.starts_with("---")).map(|offset| offset + 1);
        let block_end = close.unwrap_or(content.len());
        options.extend(content[1..block_end].iter().filter_map(|line| yaml_option(line.text)));
        return match close {
            Some(close) => &content[close + 1..],
            None => &content[content.len()..],
        };
    }
    let option_line = |line: &Line| {
        let text = line.text.trim_start();
        text.starts_with(':') && !text.starts_with(":::")
    };
    if !option_line(first) {
        return content;
    }
    let count = content.iter().take_while(|line| option_line(line)).count();
    let block: Option<Vec<(String, String)>> =
        content[..count].iter().map(|line| yaml_option(&line.text.trim_start()[1..])).collect();
    options.extend(block.unwrap_or_default());
    &content[count..]
}

fn yaml_option(text: &str) -> Option<(String, String)> {
    let (key, value) = option_pair(text.trim())?;
    let key = key.trim();
    (!key.is_empty()).then(|| (key.to_owned(), value.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{Directive, myst_markup, page_lines, rst_markup};

    type Parsed = (String, String, Vec<(String, String)>);
    type Case<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

    fn found(text: &str) -> Vec<Parsed> {
        directives(rst_markup(&page_lines(text)))
    }

    fn myst_found(text: &str) -> Vec<Parsed> {
        directives(myst_markup(text))
    }

    fn directives(markup: super::Markup) -> Vec<Parsed> {
        markup
            .directives
            .into_iter()
            .map(|Directive { name, arguments, options }| (name, arguments.join("\n"), options))
            .collect()
    }

    fn references(text: &str) -> Vec<String> {
        rst_markup(&page_lines(text)).references
    }

    fn myst_references(text: &str) -> Vec<String> {
        myst_markup(text).references
    }

    fn parsed(name: &str, argument: &str, options: &[(&str, &str)]) -> Parsed {
        let options = options.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect();
        (name.to_owned(), argument.to_owned(), options)
    }

    #[test_case(".. automodule:: pkg.mod\n", &[("automodule", "pkg.mod", &[])] ; "rst")]
    #[test_case(".. py:function:: f(a)\n", &[("py:function", "f(a)", &[])] ; "py_domain_name")]
    #[test_case(".. name ::\n", &[("name", "", &[])] ; "space_before_the_double_colon")]
    #[test_case("..  name::\n", &[("name", "", &[])] ; "two_spaces_after_the_dots")]
    #[test_case(".. AutoModule:: pkg\n", &[("AutoModule", "pkg", &[])] ; "name_keeps_its_case")]
    #[test_case(".. not a directive:: x\n", &[] ; "name_with_a_space")]
    #[test_case(".. name::arg\n", &[] ; "argument_without_a_space_after_the_colons")]
    #[test_case(".. code-block::python\n", &[] ; "code_block_without_a_space")]
    #[test_case(".. _label:\n", &[] ; "target_without_double_colon")]
    #[test_case(".. comment\n", &[] ; "comment_without_double_colon")]
    #[test_case(".. |x| image:: a.png\n", &[("image", "a.png", &[])] ; "substitution_definition")]
    #[test_case(".. |symbol|\n   image:: symbol.png\n", &[("image", "symbol.png", &[])] ; "substitution_directive_on_the_next_line")]
    #[test_case(".. |very long name,\n   split| image:: x.png\n", &[("image", "x.png", &[])] ; "substitution_name_across_lines")]
    #[test_case(".. |x| there is no directive\n", &[] ; "substitution_without_a_directive")]
    #[test_case("- .. WARNING:: text\n", &[("WARNING", "text", &[])] ; "directive_in_a_list_item")]
    #[test_case("1. .. note:: text\n", &[("note", "text", &[])] ; "arabic_enumerator")]
    #[test_case("#. .. note:: text\n", &[("note", "text", &[])] ; "auto_enumerator")]
    #[test_case("(a) .. note:: text\n", &[("note", "text", &[])] ; "parenthesised_letter")]
    #[test_case("B) .. note:: text\n", &[("note", "text", &[])] ; "upper_letter_with_a_parenthesis")]
    #[test_case("iv. .. note:: text\n", &[("note", "text", &[])] ; "lower_roman")]
    #[test_case("II) .. note:: text\n", &[("note", "text", &[])] ; "upper_roman_with_a_parenthesis")]
    #[test_case("word. .. note:: text\n", &[] ; "word_is_not_an_enumerator")]
    #[test_case("1 .. note:: text\n", &[] ; "number_without_a_closing_punctuation")]
    #[test_case("(1. .. note:: text\n", &[] ; "unclosed_parenthesis")]
    #[test_case("1.. note:: text\n", &[] ; "enumerator_without_a_space")]
    #[test_case(".. automodule::\tpkg\n", &[("automodule", "pkg", &[])] ; "tab_after_the_double_colon")]
    #[test_case("-\t.. autofunction:: pkg.a\n", &[("autofunction", "pkg.a", &[])] ; "tab_after_a_list_bullet")]
    #[test_case("..\tautofunction:: pkg.a\n", &[("autofunction", "pkg.a", &[])] ; "tab_after_the_dots")]
    #[test_case("", &[] ; "empty")]
    fn directive_forms(text: &str, expected: &[Case]) {
        let expected: Vec<Parsed> =
            expected.iter().map(|(name, argument, options)| parsed(name, argument, options)).collect();
        assert_eq!(found(text), expected);
    }

    #[test]
    fn options_under_a_directive() {
        let expected = [parsed("automodule", "pkg", &[("members", ""), ("exclude-members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n   :members:\n   :exclude-members: a, b\n"), expected);
    }

    #[test]
    fn option_value_continues_on_deeper_lines() {
        let expected = [parsed("automodule", "pkg", &[("exclude-members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n   :exclude-members: a,\n      b\n"), expected);
    }

    #[test]
    fn multi_line_argument_is_one_signature_per_line() {
        let expected = [parsed("function", "f(a,\nb)", &[("module", "pkg")])];
        assert_eq!(found(".. function:: f(a,\n   b)\n   :module: pkg\n"), expected);
    }

    #[test]
    fn signatures_on_their_own_lines_are_each_recorded() {
        let expected = [parsed("data", "IDLE\nSEND_RESPONSE", &[])];
        assert_eq!(found(".. data:: IDLE\n          SEND_RESPONSE\n"), expected);
    }

    #[test]
    fn backslash_at_a_line_end_joins_the_signatures() {
        let expected = [parsed("function", "f(a, b)", &[])];
        assert_eq!(found(".. function:: f(a,\\\n   b)\n"), expected);
    }

    #[test]
    fn options_on_the_directive_line() {
        let expected = [parsed("target-notes", "", &[("class", "custom")])];
        assert_eq!(found(".. target-notes:: :class: custom\n"), expected);
    }

    #[test]
    fn options_on_the_directive_line_end_at_a_line_of_the_block() {
        let expected = [parsed("note", "", &[("name", "mynote"), ("class", "testnote")])];
        assert_eq!(found(".. note:: :name: mynote\n   :class: testnote\n\n   Text.\n"), expected);
    }

    #[test]
    fn option_line_without_a_space_after_its_colon_is_argument_text() {
        let expected = [parsed("automodule", "pkg\n:members:x", &[])];
        assert_eq!(found(".. automodule:: pkg\n   :members:x\n"), expected);
    }

    #[test]
    fn unindented_option_line_is_not_an_option() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found(".. automodule:: pkg\n:members:\n"), expected);
    }

    #[test]
    fn text_ends_the_options() {
        let expected = [parsed("automodule", "pkg", &[("members", "")])];
        assert_eq!(found(".. automodule:: pkg\n   :members:\n\n   Text\n   :private-members:\n"), expected);
    }

    #[test]
    fn directive_after_directive_is_recorded_each() {
        let expected = [parsed("function", "a", &[]), parsed("class", "b", &[])];
        assert_eq!(found(".. function:: a\n.. class:: b\n"), expected);
    }

    #[test]
    fn directive_in_a_literal_block_is_not_recorded() {
        assert_eq!(found("Example::\n\n    .. autofunction:: pkg.hidden\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn directive_in_a_comment_is_not_recorded() {
        assert_eq!(found("..\n   .. autofunction:: pkg.hidden\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn directive_in_raw_content_is_not_recorded() {
        let expected = [parsed("raw", "html", &[])];
        assert_eq!(found(".. raw:: html\n\n   .. autofunction:: pkg.hidden\n"), expected);
    }

    #[test]
    fn options_of_a_code_directive_are_read_although_its_content_is_code() {
        let expected = [parsed("code", "python3", &[("class", "testclass")])];
        assert_eq!(found(".. code:: python3\n  :class: testclass\n\n  print(1)\n"), expected);
    }

    #[test]
    fn directive_after_a_comment_is_recorded() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found("..\n   note\n.. automodule:: pkg\n"), expected);
    }

    #[test]
    fn substitution_without_a_directive_ends_at_the_next_unindented_line() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found(".. |name|\n.. automodule:: pkg\n"), expected);
    }

    #[test]
    fn substitution_name_across_three_lines() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found(".. |a\n   b\n   c| image:: x.png\n"), expected);
    }

    #[test]
    fn grid_table_cell_directive_is_recorded() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found("+-----+------+\n| .. image:: x.png  | text |\n+-----+------+\n"), expected);
    }

    #[test]
    fn grid_table_cell_options_are_read_from_the_rows_below() {
        let expected = [parsed("automodule", "pkg", &[("members", "a")])];
        let text =
            "+----------------------+\n| .. automodule:: pkg  |\n|    :members: a      |\n+----------------------+\n";
        assert_eq!(found(text), expected);
    }

    #[test]
    fn grid_table_header_separator_keeps_the_table_open() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found("+-----+\n| a |\n+=====+\n| .. image:: x.png |\n+-----+\n"), expected);
    }

    #[test]
    fn text_after_a_grid_table_row_ends_the_table() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found("+-----+\n| a |\nText\n.. automodule:: pkg\n"), expected);
    }

    #[test]
    fn simple_table_cell_directive_is_recorded() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found("==================  =====\n.. image:: x.png    text\n==================  =====\n"), expected);
    }

    #[test]
    fn simple_table_short_row_is_cut_at_the_line_end() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found("======================  =====\n.. image:: x.png\n======================  =====\n"), expected);
    }

    #[test]
    fn simple_table_header_rule_keeps_the_table_open() {
        let expected = [parsed("image", "x.png", &[])];
        let text = "=====================  =====\nName                  Value\n====================  =====\n.. image:: x.png       text\n=====================  =====\n";
        assert_eq!(found(text), expected);
    }

    #[test]
    fn simple_table_columns_count_characters() {
        let expected = [parsed("image", "x.png", &[])];
        assert_eq!(found("=====  =====\n\u{e9}\u{e9}\u{e9}\u{e9}   .. image:: x.png\n=====  =====\n"), expected);
    }

    #[test]
    fn simple_table_ends_at_a_blank_line_after_its_bottom_border() {
        assert_eq!(found("=====  =====\nplain  text\n=====  =====\n\nplain  .. image:: x.png\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn simple_table_cell_that_is_a_comment_holds_no_directive() {
        assert_eq!(found("=====  =====\n..     text\n=====  =====\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn simple_table_row_without_a_directive_holds_none() {
        assert_eq!(found("=====  =====\nplain  text\n=====  =====\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn line_block_holds_no_directive() {
        assert_eq!(found("| .. image:: x.png  | text |\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn grid_table_ends_at_a_blank_line() {
        assert_eq!(found("+-----+\n| a |\n+-----+\n\n| .. image:: x.png\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn text_after_the_options_is_not_an_argument() {
        let expected = [parsed("automodule", "pkg", &[("members", "")])];
        assert_eq!(found(".. automodule:: pkg\n   :members:\n   text\n"), expected);
    }

    #[test]
    fn option_name_escapes_are_removed() {
        let expected = [parsed("automodule", "pkg", &[("a:b", "x")])];
        assert_eq!(found(".. automodule:: pkg\n   :a\\:b: x\n"), expected);
    }

    #[test]
    fn dots_without_a_space_are_text() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found("...\n.. automodule:: pkg\n"), expected);
    }

    #[test]
    fn footnote_is_not_a_comment() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found(".. [1] Note text\n.. automodule:: pkg\n"), expected);
    }

    #[test]
    fn double_colon_line_is_argument_text() {
        let expected = [parsed("automodule", "pkg\n:: text", &[])];
        assert_eq!(found(".. automodule:: pkg\n   :: text\n"), expected);
    }

    #[test]
    fn tab_separates_an_option_name_from_its_value() {
        let expected = [parsed("automodule", "pkg", &[("members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n   :members:\ta, b\n"), expected);
    }

    #[test]
    fn tab_indented_option_continues_on_a_deeper_line() {
        let expected = [parsed("automodule", "pkg", &[("members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n\t:members: a,\n\t\tb\n"), expected);
    }

    #[test]
    fn tab_is_eight_columns_when_it_meets_spaces() {
        let expected = [parsed("automodule", "pkg", &[("members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n    :members: a,\n\tb\n"), expected);
    }

    #[test]
    fn list_item_directive_options_are_read_from_its_column() {
        let expected = [parsed("automodule", "pkg", &[])];
        assert_eq!(found("- .. automodule:: pkg\n  :members: x\n"), expected);
    }

    #[test]
    fn list_item_directive_options_deeper_than_its_column_are_read() {
        let expected = [parsed("automodule", "pkg", &[("members", "x")])];
        assert_eq!(found("- .. automodule:: pkg\n     :members: x\n"), expected);
    }

    #[test]
    fn list_item_comment_ends_at_its_column() {
        let expected = [parsed("automodule", "pkg.a", &[])];
        assert_eq!(found("- ..\n  note\n  .. automodule:: pkg.a\n"), expected);
    }

    #[test]
    fn list_item_comment_content_is_not_recorded() {
        assert_eq!(found("- ..\n    .. autofunction:: pkg.hidden\n"), Vec::<Parsed>::new());
    }

    #[test]
    fn list_item_splits_the_paragraph_before_it() {
        assert_eq!(references("- *a\n- :func:`pkg.a` b*\n"), ["pkg.a"]);
    }

    #[test]
    fn field_splits_the_paragraph_before_it() {
        assert_eq!(references(":a: *x\n:b: :func:`pkg.a` y*\n"), ["pkg.a"]);
    }

    #[test]
    fn grid_cell_role_spanning_rows_keeps_no_border() {
        let found = references("+---------------+\n| :func:`pkg.   |\n| a`            |\n+---------------+\n");
        let targets: Vec<String> =
            found.iter().map(|target| target.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
        assert_eq!(targets, ["pkg. a"]);
    }

    #[test]
    fn grid_cell_roles_do_not_join_across_cells() {
        assert_eq!(references("+-----+-----+\n| :func:`a | b`\n+-----+-----+\n"), Vec::<String>::new());
    }

    #[test]
    fn multibyte_text_before_a_grid_bar_keeps_the_cell_split() {
        assert_eq!(references("+---+---+\n| \u{e9} | :func:`pkg.a` |\n+---+---+\n"), ["pkg.a"]);
    }

    #[test]
    fn nested_note_fences_are_read_without_recursion() {
        let depth = 20_000;
        let text = format!("{}```{{automodule}} pkg.a\n{}", "```{note}\n".repeat(depth), "```\n".repeat(depth));
        let found = myst_found(&text);
        assert_eq!(found.len(), depth + 1);
        assert_eq!(found.last(), Some(&parsed("automodule", "pkg.a", &[])));
    }

    #[test]
    fn nested_colon_fences_are_read_without_recursion() {
        let depth = 20_000;
        let text = format!("{}:::{{automodule}} pkg.a\n{}", ":::{note}\n".repeat(depth), ":::\n".repeat(depth));
        let found = myst_found(&text);
        assert_eq!(found.len(), depth + 1);
        assert_eq!(found.last(), Some(&parsed("automodule", "pkg.a", &[])));
    }

    #[test_case("```{automodule} pkg.mod\n```\n", &[("automodule", "pkg.mod", &[])] ; "backtick_fence")]
    #[test_case("~~~{autofunction} pkg.f\n~~~\n", &[("autofunction", "pkg.f", &[])] ; "tilde_fence")]
    #[test_case(":::{automodule} pkg.mod\n:::\n", &[("automodule", "pkg.mod", &[])] ; "colon_fence")]
    #[test_case("```{automodule} pkg\n---\nmembers:\nexclude-members: a, b\n---\n", &[("automodule", "pkg", &[("members", ""), ("exclude-members", "a, b")])] ; "yaml_options_block")]
    #[test_case("```{automodule} pkg\n:members:\n:exclude-members: a,\n  b\n```\n", &[("automodule", "pkg", &[("members", ""), ("exclude-members", "a,")])] ; "option_lines_keep_one_line_each")]
    #[test_case("``{autofunction} pkg.f\n``\n", &[] ; "two_backtick_fence")]
    #[test_case("::{autofunction} pkg.f\n::\n", &[] ; "two_colon_fence")]
    #[test_case("    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "fence_indented_four_spaces")]
    #[test_case("`{autofunction}` pkg.hidden is a directive\n", &[] ; "inline_code_at_line_start")]
    #[test_case("```{autofunction} pkg.f`x`\n```\n", &[] ; "backtick_fence_info_with_a_backtick")]
    #[test_case("```{code-block} python\nx = 1\n```\n", &[("code-block", "python", &[])] ; "code_block_is_recorded_without_its_content")]
    #[test_case("```python\n{automodule} pkg\n```\n", &[] ; "plain_code_fence")]
    #[test_case("```{eval-rst}\n.. autofunction:: pkg.f\n   :noindex:\n```\n", &[("eval-rst", "", &[]), ("autofunction", "pkg.f", &[("noindex", "")])] ; "rst_directive_in_eval_rst")]
    #[test_case("```{automodule} pkg\n---\nmembers:\n```\n", &[("automodule", "pkg", &[("members", "")])] ; "yaml_options_block_without_its_end")]
    #[test_case("```{automodule} pkg\n---\nmembers\n---\n", &[("automodule", "pkg", &[])] ; "yaml_line_without_a_colon_is_dropped")]
    #[test_case("```{automodule pkg\n```\n", &[] ; "directive_name_without_a_closing_brace")]
    #[test_case("- item\n\n    ```{automodule} pkg\n    ```\n", &[("automodule", "pkg", &[])] ; "four_spaces_in_a_list_item_are_not_code")]
    #[test_case("- item\n\n        ```{automodule} pkg\n        ```\n", &[] ; "eight_spaces_in_a_list_item_are_code")]
    #[test_case("1. item\n\n   ```{automodule} pkg\n   ```\n", &[("automodule", "pkg", &[])] ; "ordered_item_content_starts_after_the_marker")]
    #[test_case("- a\n  - b\n\n      ```{automodule} pkg\n      ```\n", &[("automodule", "pkg", &[])] ; "nested_item_content_indent")]
    #[test_case("- item\n\nText\n\n    ```{automodule} pkg\n    ```\n", &[] ; "indentation_after_a_paragraph_outside_the_list_is_code")]
    #[test_case("* * *\n\n    ```{automodule} pkg\n    ```\n", &[] ; "asterisk_thematic_break_is_not_a_list_item")]
    #[test_case("- - -\n\n    ```{automodule} pkg\n    ```\n", &[] ; "dash_thematic_break_is_not_a_list_item")]
    #[test_case("---\n\n    ```{automodule} pkg\n    ```\n", &[] ; "dashes_thematic_break_is_not_a_list_item")]
    #[test_case("- item\nlazy\n\n    ```{automodule} pkg\n    ```\n", &[("automodule", "pkg", &[])] ; "lazy_continuation_keeps_the_list_item")]
    #[test_case("-item\n\n    ```{automodule} pkg\n    ```\n", &[] ; "bullet_without_a_space_is_not_a_list_item")]
    #[test_case("```{automodule} pkg\n    ```\n", &[("automodule", "pkg", &[])] ; "closing_fence_indented_four_spaces_does_not_close")]
    #[test_case("```{automodule} pkg\n```\n\n```{autofunction} f\n```\n", &[("automodule", "pkg", &[]), ("autofunction", "f", &[])] ; "blank_line_between_fences")]
    #[test_case("```{note}\n```{automodule} pkg\n```\n", &[("note", "", &[]), ("automodule", "pkg", &[])] ; "directive_in_a_note_fence_is_recorded_after_it")]
    #[test_case("- ```{automodule} pkg\n  ```\n", &[("automodule", "pkg", &[])] ; "fence_on_a_list_marker_line")]
    #[test_case("1. :::{automodule} pkg\n   :::\n", &[("automodule", "pkg", &[])] ; "colon_fence_on_an_ordered_marker_line")]
    #[test_case("> ```{automodule} pkg\n> ```\n", &[("automodule", "pkg", &[])] ; "fence_in_a_blockquote")]
    #[test_case("> ```{eval-rst}\n> .. autofunction:: pkg.a\n> ```\n", &[("eval-rst", "", &[]), ("autofunction", "pkg.a", &[])] ; "eval_rst_in_a_blockquote")]
    #[test_case("- item\n# Heading\n\n    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "heading_ends_the_list_item")]
    #[test_case("# Title\n    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "indented_fence_after_a_heading_is_code")]
    #[test_case("- item\n***\n\n    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "thematic_break_ends_the_list_item")]
    #[test_case("- item\n<div>\n\n    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "html_block_ends_the_list_item")]
    #[test_case("- item\n> quote\n\n    ```{autofunction} pkg.hidden\n    ```\n", &[] ; "blockquote_ends_the_list_item")]
    #[test_case("```{automodule} pkg\n\t:members: a\n```\n", &[("automodule", "pkg", &[("members", "a")])] ; "tab_indented_option_is_read")]
    #[test_case("```{automodule} pkg\n  :members:   a, b\n:exclude-members:c\n```\n", &[("automodule", "pkg", &[])] ; "option_block_with_an_invalid_line_is_not_read")]
    #[test_case("```{automodule} pkg\n---\nmembers: a\n---\n```\n", &[("automodule", "pkg", &[("members", "a")])] ; "yaml_options_are_read_after_the_front_matter_closes")]
    fn fences(text: &str, expected: &[Case]) {
        let expected: Vec<Parsed> =
            expected.iter().map(|(name, argument, options)| parsed(name, argument, options)).collect();
        assert_eq!(myst_found(text), expected);
    }

    #[test]
    fn front_matter_is_not_a_directive_or_a_role_source() {
        assert_eq!(myst_references("---\ntitle: x\n---\n{func}`pkg.a`\n"), ["pkg.a"]);
    }

    #[test_case("% {func}`pkg.hidden`\n", &[] ; "percent_comment")]
    #[test_case("<div>\n{func}`pkg.hidden`\n</div>\n", &[] ; "html_block")]
    #[test_case("<http://{func}`pkg.hidden`>\n", &[] ; "autolink")]
    #[test_case("<span title=\"{func}`pkg.hidden`\">x</span>\n", &[] ; "inline_html_attribute")]
    #[test_case("[x](<{func}`pkg.hidden`>)\n", &[] ; "link_destination")]
    #[test_case("[a]: http://x '{func}`pkg.hidden`'\n", &[] ; "reference_definition")]
    #[test_case("[{func}`pkg.a`](http://x)\n", &["pkg.a"] ; "role_in_link_text")]
    #[test_case("> {func}`pkg.a`\n", &["pkg.a"] ; "role_in_blockquote")]
    #[test_case("# {func}`pkg.a`\n", &["pkg.a"] ; "role_in_heading")]
    #[test_case("- item\n# {func}`pkg.a`\n", &["pkg.a"] ; "role_after_a_heading_that_ends_the_item")]
    fn myst_roles_outside_prose(text: &str, expected: &[&str]) {
        assert_eq!(myst_references(text), expected);
    }

    #[test]
    fn myst_lazy_continuation_keeps_a_role_in_the_item() {
        assert_eq!(myst_references("- a\nb {func}`pkg.a`\n"), ["pkg.a"]);
    }

    #[test]
    fn long_myst_page_reads_in_linear_time() {
        let text = "{func}`pkg.a` line\n".repeat(10_000);
        let started = std::time::Instant::now();
        assert_eq!(myst_references(&text).len(), 10_000);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
}
