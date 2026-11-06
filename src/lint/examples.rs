// The examples of a rule page, read from its explanation. The tests lint them, the docs render them, `simplint rule`
// shows them and `simplint rules` lists them, so every reader parses a page the same way.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Report,
    Ok,
    Context,
    Fixed,
}

impl Role {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "report" => Some(Self::Report),
            "ok" => Some(Self::Ok),
            "context" => Some(Self::Context),
            "fixed" => Some(Self::Fixed),
            _ => None,
        }
    }
}

// A fenced block: its lines in the explanation, 1-based and inclusive, and its source without the fence's indentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fence {
    pub line: usize,
    pub end_line: usize,
    pub source: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Block {
    pub role: Role,
    // `python`, or the markup of a page the project's documentation holds, such as `rst`, which only gives context.
    pub language: String,
    // `example.py` unless the block says `file=`.
    pub file: String,
    pub named: bool,
    #[serde(flatten)]
    pub fence: Fence,
}

// An optional `pyproject.toml` followed by the files of one project. The Example section shares its config with the
// examples after it, so that config can sit above the example's `line`.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Example {
    // The `##` heading the example sits under.
    pub section: Option<String>,
    pub line: usize,
    pub end_line: usize,
    pub config: Option<Fence>,
    pub blocks: Vec<Block>,
}

// A rule's examples, or the error that keeps them from parsing, under one key of the rules listing.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Examples {
    Parsed(Vec<Example>),
    Error(String),
}

impl From<Result<Vec<Example>, String>> for Examples {
    fn from(parsed: Result<Vec<Example>, String>) -> Self {
        parsed.map_or_else(Self::Error, Self::Parsed)
    }
}

// Blocks join the example before them unless prose or another fence comes between, and a block joins only when it and
// the block before it both name their file. A `fixed` block must follow the `report` of its file. The Example
// section shows the flagged code and its replacement apart, so a config there holds for the rest of the section.
pub fn parse(explanation: &str) -> Result<Vec<Example>, String> {
    let mut examples: Vec<Example> = Vec::new();
    let mut open: Option<usize> = None;
    let mut section: Option<&str> = None;
    let mut section_config: Option<Fence> = None;
    let mut lines = explanation.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        let trimmed = line.trim_start();
        let Some(info) = trimmed.strip_prefix("```") else {
            if let Some(heading) = line.strip_prefix("## ") {
                section = Some(heading);
                section_config = None;
            }
            if !line.trim().is_empty() {
                open = None;
            }
            continue;
        };
        let indent = line.len() - trimmed.len();
        let mut source = String::new();
        let mut end_line = None;
        for (inner_number, inner) in lines.by_ref() {
            if inner.trim() == "```" {
                end_line = Some(inner_number + 1);
                break;
            }
            let leading = inner.len() - inner.trim_start_matches(' ').len();
            source.push_str(&inner[leading.min(indent)..]);
            source.push('\n');
        }
        let Some(end_line) = end_line else {
            return Err(format!("line {}: unclosed code block", number + 1));
        };
        let fence = Fence { line: number + 1, end_line, source };
        let tokens: Vec<&str> = info.split_whitespace().collect();
        match tokens.as_slice() {
            ["toml", "config"] => {
                if section == Some("Example") {
                    section_config = Some(fence.clone());
                }
                examples.push(Example {
                    section: section.map(str::to_owned),
                    line: fence.line,
                    end_line,
                    config: Some(fence),
                    blocks: Vec::new(),
                });
                open = Some(examples.len() - 1);
            }
            [language @ ("python" | "rst" | "markdown"), role, attributes @ ..] => {
                let role = Role::parse(role).ok_or_else(|| format!("line {}: unknown role `{role}`", number + 1))?;
                if *language != "python" && (role != Role::Context || attributes.is_empty()) {
                    return Err(format!("line {}: a `{language}` block is a `context` block with a file", number + 1));
                }
                let file = match attributes {
                    [] => None,
                    [attribute] => Some(
                        attribute
                            .strip_prefix("file=")
                            .ok_or_else(|| format!("line {}: unknown attribute `{attribute}`", number + 1))?,
                    ),
                    _ => return Err(format!("line {}: more than one attribute", number + 1)),
                };
                let block = Block {
                    role,
                    language: (*language).to_owned(),
                    file: file.unwrap_or("example.py").to_owned(),
                    named: file.is_some(),
                    fence,
                };
                let index = push(&mut examples, open, block, section)
                    .map_err(|reason| format!("line {}: {reason}", number + 1))?;
                examples[index].end_line = end_line;
                examples[index].config = examples[index].config.take().or_else(|| section_config.clone());
                open = Some(index);
            }
            _ => open = None,
        }
    }
    match examples.iter().position(|example| example.blocks.is_empty()) {
        Some(index) => Err(format!("example {}: a config block has no python block after it", index + 1)),
        None => Ok(examples),
    }
}

// Returns the index of the example the block landed in.
fn push(
    examples: &mut Vec<Example>,
    open: Option<usize>,
    block: Block,
    section: Option<&str>,
) -> Result<usize, &'static str> {
    if block.role == Role::Fixed {
        let index = open
            .filter(|&index| {
                examples[index]
                    .blocks
                    .last()
                    .is_some_and(|previous| previous.role == Role::Report && previous.file == block.file)
            })
            .ok_or("`fixed` must directly follow the `report` block of its file")?;
        examples[index].blocks.push(block);
        return Ok(index);
    }
    if let Some(index) = open
        && examples[index].blocks.last().is_none_or(|previous| previous.named && block.named)
    {
        examples[index].blocks.push(block);
        return Ok(index);
    }
    examples.push(Example {
        section: section.map(str::to_owned),
        line: block.fence.line,
        end_line: block.fence.end_line,
        config: None,
        blocks: vec![block],
    });
    Ok(examples.len() - 1)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use test_case::test_case;

    use super::{Examples, parse};

    #[test_case("```python report\nX = 1\n", "line 1: unclosed code block" ; "unclosed_block")]
    #[test_case("```python loud\nX = 1\n```\n", "line 1: unknown role `loud`" ; "unknown_role")]
    #[test_case("```python report size=2\n```\n", "line 1: unknown attribute `size=2`" ; "unknown_attribute")]
    #[test_case("```python report file=a.py extra\n```\n", "line 1: more than one attribute" ; "two_attributes")]
    #[test_case("```python fixed\nX = 1\n```\n", "line 1: `fixed` must directly follow the `report` block of its file" ; "fixed_first")]
    #[test_case(
        "```python report\nX = 1\n```\n\n```python fixed file=a.py\nX = 1\n```\n",
        "line 5: `fixed` must directly follow the `report` block of its file" ;
        "fixed_of_other_file"
    )]
    #[test_case("```toml config\n[tool]\n```\n", "example 1: a config block has no python block after it" ; "config_alone")]
    #[test_case("```rst ok file=a.rst\nA\n```\n", "line 1: a `rst` block is a `context` block with a file" ; "markup_not_context")]
    #[test_case("```rst context\nA\n```\n", "line 1: a `rst` block is a `context` block with a file" ; "markup_without_file")]
    fn parse_rejects_malformed_examples(explanation: &str, reason: &str) {
        assert_eq!(parse(explanation), Err(reason.to_owned()));
    }

    #[test]
    fn listing_reports_the_parse_error() {
        let listed = serde_json::to_value(Examples::from(parse("```python loud\nX = 1\n```\n"))).expect("json");
        assert_eq!(listed, json!({"error": "line 1: unknown role `loud`"}));
    }
}
