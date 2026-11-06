use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::layout::ends_in_url;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::{is_tool_directive, own_line_comment_runs, owns_docstring};
use crate::rules::{docstring, indentation};
use crate::syntax::for_each_body;

// One line of a comment or docstring: the span a report and a rewrap cover, and what the rewrap needs of the text.
struct Line<'a> {
    range: TextRange,
    words: Vec<&'a str>,
    // Where the text starts on its line, and how wide the whole line is, in characters.
    column: usize,
    width: usize,
    // Prose lines of one paragraph are rewrapped together; any other line stays as it is.
    prose: bool,
    // Lines never reported: a directive, a single word, or a URL ending the line, as Ruff's E501 leaves them.
    exempt: bool,
    // Whether a docstring's closing quotes follow this line's text.
    ends_string: bool,
}

// Comment and docstring lines longer than the comment width. Ruff's line length does not wrap them, and `ruff format`
// leaves them alone.
pub fn long_comment_line(checker: &Checker) {
    let limit = checker.settings.from_ruff.comment_width;
    let bracketed = bracketed_columns(checker);
    for unit in comment_units(checker, limit, &bracketed) {
        report(checker, limit, "Comment", &unit, |lines| comment_fix(checker, limit, lines));
    }
    for_each_body(checker.body(), &mut |body, owner| {
        let Some(literal) = docstring(body).filter(|_| owns_docstring(owner)) else {
            return;
        };
        // A line the author silenced with `noqa` or `ruff: ignore` is left alone: rewrapping its docstring would change
        // the line the directive covers, which leaves the directive unused.
        if checker.trailing_comment(literal.range).is_some_and(|comment| is_tool_directive(checker.text(comment))) {
            return;
        }
        let [part] = literal.value.as_slice() else {
            return;
        };
        let indent = indentation(checker.source, body[0].range());
        let quotes = &checker.source[part.content_range().end().to_usize()..literal.range.end().to_usize()];
        for unit in docstring_units(checker, limit, indent, quotes, part.content_range()) {
            report(checker, limit, "Docstring", &unit, |lines| {
                docstring_fix(checker, limit, indent, (quotes, literal.range.end()), lines)
            });
        }
    });
}

// Reports the first long line of a unit, once for the whole paragraph, with the rewrapped paragraph as its fix. Ruff's
// E501 and W505 already report every long line, so a line this rule cannot rewrap is left to them.
fn report(checker: &Checker, limit: usize, kind: &str, unit: &[Line], reflow: impl Fn(&[Line]) -> Option<Fix>) {
    let Some(long) = unit.iter().find(|line| !line.exempt && line.width > limit) else {
        return;
    };
    // A file-level `noqa` may silence the very lines a reflow shortens, which would leave its code unused.
    let fix = if checker.state().has_file_noqa(checker) { None } else { unit[0].prose.then(|| reflow(unit)).flatten() };
    let Some(fix) = fix else {
        return;
    };
    checker.report_fix(
        Rule::LongCommentLine,
        long.range,
        format!("{kind} line is longer than {limit} characters"),
        Some(fix),
        "Reflow the paragraph",
    );
}

// Own-line comments in runs of prose lines that continue one another at the same column. Other lines form units of
// their own.
fn comment_units<'a>(
    checker: &Checker<'a>,
    limit: usize,
    bracketed: &FxHashMap<TextSize, usize>,
) -> Vec<Vec<Line<'a>>> {
    let mut units = Vec::new();
    for run in own_line_comment_runs(checker) {
        let mut prose_run: Vec<Line<'a>> = Vec::new();
        for comment in run {
            let line = comment_line(checker, comment, limit, bracketed);
            if !line.prose {
                push_run(&mut units, &mut prose_run);
                units.push(vec![line]);
                continue;
            }
            let continues = prose_run
                .last()
                .is_some_and(|previous| previous.column == line.column && !is_broken_early(previous, &line, limit));
            if !continues {
                push_run(&mut units, &mut prose_run);
            }
            prose_run.push(line);
        }
        push_run(&mut units, &mut prose_run);
    }
    units
}

fn push_run<'a>(units: &mut Vec<Vec<Line<'a>>>, run: &mut Vec<Line<'a>>) {
    if !run.is_empty() {
        units.push(std::mem::take(run));
    }
}

// An own-line comment, at the column `ruff format` puts it.
fn comment_line<'a>(
    checker: &Checker<'a>,
    comment: TextRange,
    limit: usize,
    bracketed: &FxHashMap<TextSize, usize>,
) -> Line<'a> {
    let source = checker.source;
    let text = checker.text(comment).trim_end();
    let indent = bracketed.get(&comment.start()).copied().unwrap_or_else(|| {
        formatted_indent(checker, &source[source.line_start(comment.start()).to_usize()..comment.start().to_usize()])
    });
    // The formatter puts a space after a `#` that runs into the text, unless the text starts with one of its markers.
    let spaced = !text[1..].is_empty() && !text[1..].starts_with([' ', '!', ':', '#', '\'', '|', '\u{A0}']);
    let width = indent + text.chars().count() + usize::from(spaced);
    let rest = text[1..].strip_prefix(' ').unwrap_or(&text[1..]);
    let directive = is_tool_directive(text);
    let prose = !rest.is_empty()
        && !rest.starts_with(char::is_whitespace)
        && !text.starts_with("##")
        && !directive
        && is_prose_text(rest);
    let words = rest.split_whitespace().collect::<Vec<_>>();
    Line {
        range: comment,
        exempt: directive || text.starts_with("#!") || is_exempt(&words, width, limit),
        words,
        column: indent + 2,
        width,
        prose,
        ends_string: false,
    }
}

// The column of each own-line comment inside brackets. `ruff format` splits every bracket around such a comment and
// indents the comment one level for each, below the indentation of its statement's block, wherever the source has it.
fn bracketed_columns(checker: &Checker) -> FxHashMap<TextSize, usize> {
    let indent_width = checker.settings.from_ruff.indent_width;
    let (mut depth, mut level) = (0_usize, 0_usize);
    let mut columns = FxHashMap::default();
    for token in checker.parsed.tokens() {
        match token.kind() {
            TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => depth += 1,
            TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => depth = depth.saturating_sub(1),
            TokenKind::Indent => level += 1,
            TokenKind::Dedent => level = level.saturating_sub(1),
            TokenKind::Comment if depth > 0 && !has_leading_content(token.start(), checker.source) => {
                columns.insert(token.start(), indent_width * (level + depth));
            }
            _ => {}
        }
    }
    columns
}

// The indentation `ruff format` gives a line. It re-indents a file to its own indent width, in spaces, so a file
// indented by two spaces or by tabs gets four spaces per level, and its lines are wider after the format; the lines
// are judged at that width.
fn formatted_indent(checker: &Checker, prefix: &str) -> usize {
    let unit = checker.stylist.indentation().as_str().chars().count().max(1);
    let indent = prefix.chars().take_while(|char| char.is_whitespace()).count();
    let rest = prefix.chars().count() - indent;
    indent / unit * checker.settings.from_ruff.indent_width + indent % unit + rest
}

// The lines of each docstring paragraph. A blank line ends a paragraph, and a line that is not prose stands alone.
// `ruff format` moves the lines after the first by the same amount, so that the least indented one sits at the
// docstring's indentation, and puts the closing quotes of a docstring with one line of text after that text; the
// lines are judged as it lays them out. A docstring with an escaped line break it leaves as it is.
fn docstring_units<'a>(
    checker: &Checker<'a>,
    limit: usize,
    indent: &str,
    quotes: &str,
    content: TextRange,
) -> Vec<Vec<Line<'a>>> {
    let laid = laid_out_lines(checker, indent, quotes, content);
    // Only a line over the limit is reported, so a docstring without one needs no units.
    if !laid.iter().flatten().any(|line| line.width > limit) {
        return Vec::new();
    }
    let mut units = Vec::new();
    let mut run: Vec<Line<'a>> = Vec::new();
    let mut in_doctest = false;
    for piece in laid {
        let Some(LaidOut { text: trimmed, range, column, aligned, width, ends_string }) = piece else {
            in_doctest = false;
            push_run(&mut units, &mut run);
            continue;
        };
        // A doctest's expected output runs from its `>>>` line to the next blank line, and must stay as printed.
        in_doctest |= trimmed.starts_with(">>>");
        let words = trimmed.split_whitespace().collect::<Vec<_>>();
        let prose = aligned && !in_doctest && is_prose_text(trimmed) && !trimmed.contains('\\');
        let line = Line { range, exempt: is_exempt(&words, width, limit), column, width, prose, ends_string, words };
        if prose && run.last().is_some_and(|previous| !is_broken_early(previous, &line, limit)) {
            run.push(line);
        } else {
            push_run(&mut units, &mut run);
            if prose {
                run.push(line);
            } else {
                units.push(vec![line]);
            }
        }
    }
    push_run(&mut units, &mut run);
    units
}

// Each line of a docstring with its text, column and width as the formatter lays it out, or `None` for a blank line.
fn laid_out_lines<'a>(
    checker: &Checker<'a>,
    indent: &str,
    quotes: &str,
    content: TextRange,
) -> Vec<Option<LaidOut<'a>>> {
    let source = checker.source;
    let text = checker.text(content);
    let laid_out = !has_escaped_line_break(text);
    let one_line = laid_out && text.split_once('\n').is_none_or(|(_, rest)| rest.trim().is_empty());
    let stripped =
        text.split('\n').skip(1).filter(|piece| !piece.trim().is_empty()).map(indent_columns).min().unwrap_or_default();
    let docstring_column = formatted_indent(checker, indent);
    let base = indent_columns(source.full_line_str(content.start()));
    let mut offset = content.start();
    let laid: Vec<Option<LaidOut<'a>>> = text
        .split('\n')
        .map(|piece| {
            let start = offset;
            offset += TextSize::of(piece) + TextSize::of("\n");
            let trimmed = piece.trim();
            if trimmed.is_empty() {
                return None;
            }
            let leading = piece.len() - piece.trim_start().len();
            let text_start = start + TextSize::of(&piece[..leading]);
            let text_end = text_start + TextSize::of(trimmed);
            let first = start == content.start();
            let ends_string = text_end == content.end() || (one_line && first);
            let (column, aligned) = if first {
                (formatted_indent(checker, &source[source.line_start(start).to_usize()..text_start.to_usize()]), true)
            } else if laid_out {
                (docstring_column + indent_columns(piece) - stripped, indent_columns(piece) == stripped)
            } else {
                (formatted_indent(checker, &piece[..leading]), indent_columns(piece) == base)
            };
            let width = column + trimmed.chars().count() + if ends_string { quotes.chars().count() } else { 0 };
            Some(LaidOut {
                text: trimmed,
                range: TextRange::new(text_start, text_end),
                column,
                aligned,
                width,
                ends_string,
            })
        })
        .collect();
    laid
}

// A docstring line as the formatter lays it out: its text, where it starts and how wide it is, whether it sits at the
// column of the paragraph around it, and whether the closing quotes follow it.
struct LaidOut<'a> {
    text: &'a str,
    range: TextRange,
    column: usize,
    aligned: bool,
    width: usize,
    ends_string: bool,
}

// The width of a line's leading whitespace, with a tab stop every eight columns, as the formatter measures it.
fn indent_columns(line: &str) -> usize {
    line.chars()
        .take_while(|char| char.is_whitespace())
        .fold(0, |column, char| if char == '\t' { column / 8 * 8 + 8 } else { column + 1 })
}

fn has_escaped_line_break(text: &str) -> bool {
    text.split('\\').skip(1).any(|after| after.trim_start_matches([' ', '\t']).starts_with(['\n', '\r']))
}

// Prose starts with a word, not with a marker of structure: a list, a field, a doctest, a heading underline, a
// directive or the header of a literal block. A run of spaces inside the text aligns the columns of a table.
fn is_prose_text(text: &str) -> bool {
    !text.starts_with(|char: char| char.is_ascii_digit() || "-*+>.:|=~^#(!".contains(char))
        && !text.ends_with("::")
        && !text.chars().all(|char| "-=~^*+#_".contains(char))
        && !text.contains("   ")
}

// A line that had room for the next line's first word was broken on purpose, as a label above a sentence or one
// clause per line, so it ends its paragraph.
fn is_broken_early(previous: &Line, line: &Line, limit: usize) -> bool {
    line.words.first().is_some_and(|word| previous.width + 1 + word.chars().count() <= limit)
}

// A line Ruff's E501 leaves alone: a single word, or a URL that ends it and starts within the limit.
fn is_exempt(words: &[&str], width: usize, limit: usize) -> bool {
    match words {
        [] | [_] => true,
        [.., last] => ends_in_url(last, width, limit),
    }
}

// The paragraph's comment lines, rewrapped at the comment width under the same indentation.
fn comment_fix(checker: &Checker, limit: usize, lines: &[Line]) -> Option<Fix> {
    let room = limit.checked_sub(lines[0].column)?;
    let wrapped = wrap(&words(lines), room, room)?;
    let indent = indentation(checker.source, lines[0].range);
    let eol = checker.stylist.line_ending().as_str();
    let text = wrapped.iter().map(|line| format!("# {line}")).collect::<Vec<_>>().join(&format!("{eol}{indent}"));
    Some(Fix::safe_edit(Edit::range_replacement(
        text,
        TextRange::new(lines[0].range.start(), lines.last()?.range.end()),
    )))
}

// The paragraph's text rewrapped inside the docstring. The first line starts after the opening quotes, so it has the
// room left after its column; the others sit at the docstring's indentation. A closing quote joins the last word, so
// it counts toward the width. Changing the words changes `__doc__`, so the fix is unsafe.
fn docstring_fix(
    checker: &Checker,
    limit: usize,
    indent: &str,
    (quotes, end): (&str, TextSize),
    lines: &[Line],
) -> Option<Fix> {
    // A single-quoted docstring cannot hold the line break a rewrap adds.
    if quotes.len() < 3 {
        return None;
    }
    let mut words = words(lines);
    let last = lines.last()?;
    let range_end = if last.ends_string {
        words.last_mut()?.push_str(quotes);
        end
    } else {
        last.range.end()
    };
    // A paragraph below the first keeps the indentation it has, which the formatter moves with the docstring's other
    // lines, so its lines get the room its first line has.
    let source = checker.source;
    let opening =
        !source[TextRange::new(source.line_start(lines[0].range.start()), lines[0].range.start())].trim().is_empty();
    let (indent, rest) =
        if opening { (indent, indent.chars().count()) } else { (indentation(source, lines[0].range), lines[0].column) };
    let wrapped = wrap(&words, limit.checked_sub(lines[0].column)?, limit.checked_sub(rest)?)?;
    let eol = checker.stylist.line_ending().as_str();
    let text = wrapped.join(&format!("{eol}{indent}"));
    Some(Fix::unsafe_edit(Edit::range_replacement(text, TextRange::new(lines[0].range.start(), range_end))))
}

fn words(lines: &[Line]) -> Vec<String> {
    lines.iter().flat_map(|line| line.words.iter().map(|word| (*word).to_owned())).collect()
}

// Fills lines greedily: the first gets `first` characters and the rest `rest`. `None` when a word is wider than the
// line it must start, since it would then stay over the limit.
fn wrap(words: &[String], first: usize, rest: usize) -> Option<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    for word in words {
        let size = word.chars().count();
        let count = lines.len();
        let room = if count == 1 { first } else { rest };
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + size <= room => {
                line.push(' ');
                line.push_str(word);
            }
            _ => {
                if size > if count == 0 { first } else { rest } {
                    return None;
                }
                lines.push(word.clone());
            }
        }
    }
    Some(lines)
}
